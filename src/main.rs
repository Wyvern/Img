mod util;
use {std::*, util::*};

static SEP: &str = " | ";
static CSS: [&str; 3] = ["url(", "image(", "image-set("];
static JSON: sync::OnceLock<serde_json::Value> = sync::OnceLock::new();
static CURL: [&str; if cfg!(debug_assertions) { 7 } else { 6 }] = [
    "--compressed",
    "-kfsL",
    "-A",
    "Mozilla/5.0 Firefox/Edge/Chrome",
    "--tcp-fastopen",
    "--tcp-nodelay",
    #[cfg(debug_assertions)]
    "-S",
    // "-OJ",
];

fn check_args() -> (String, String) {
    let mut args;
    #[cfg(test)]
    {
        args = env::args().skip(3);
    }
    #[cfg(not(test))]
    {
        args = env::args();
    };
    if args.len() > 3 {
        quit!("Too many arguments.\nUsage: {}", "Img <url> [dir]");
    }
    let url = args.nth(1).unwrap_or_else(|| {
        quit!("Please input <url> argument.");
    });
    let dir = args.next().unwrap_or_default();
    (url, dir)
}

fn main() {
    let (url, dir) = check_args();

    if !dir.is_empty() {
        let path = path::Path::new(&dir);
        if path.exists() && path.is_dir() {
            env::set_current_dir(path)
                .unwrap_or_else(|x| quit!("Change working directory to {} failed: {} !", &dir, x))
        } else {
            quit!("The path {} is invalid!", &dir)
        }
    }

    check_host(&url);

    let mut next_page = parse(&url);
    if cfg!(not(test)) {
        while !next_page.is_empty() {
            next_page = parse(&next_page);
        }
    }
}

///Get `scheme` and `host` info from valid url string
fn check_host(addr: &str) -> &str {
    let (scheme, rest) = addr.split_once("://").unwrap_or(("http", addr));

    if ["http", "https"]
        .iter()
        .all(|x| !scheme.trim().eq_ignore_ascii_case(x))
    {
        quit!("Scheme {} is NOT valid {} protocol.", scheme, "http(s)");
    }

    let host = &rest[..rest.find('/').unwrap_or(rest.len())];
    if !host.contains('.') {
        quit!("Invalid web host name: {}", host);
    }
    host
}

///Get `host` info and Generate `img/next/album` selector data
fn host_info(host: &str) -> [Option<&str>; 4] {
    let site = JSON
        .get_or_init(website)
        .as_object()
        .expect("`web.json` file parse error!")["Sites"]
        .as_array()
        .expect("Parse `Sites` in `web.json` key error!")
        .iter()
        .find(|&site| {
            site["Site"].as_str().is_some_and(|s| {
                s.split_terminator(',').any(|s| {
                    let mut parts = host.rsplit('.').take(2).collect::<Vec<_>>();
                    parts.reverse();
                    let r = parts.join(".").eq_ignore_ascii_case(s.trim());
                    if r {
                        tdbg!(s);
                    }
                    r
                })
            })
        });

    site.map_or([None; 4], |s| {
        ["Img", "Next", "Album", "Title"].map(|key| s[key].as_str().map(|v| v.trim()))
    })
}

///Fetch web page generate html content
fn get_html(addr: &str) -> (String, usize) {
    use sync::mpsc::*;
    let (s, r) = channel();
    _ = io::stdout().lock();
    let h = thread::spawn(|| {
        circle_indicator(r);
    });
    let out = process::Command::new("curl")
        .args(CURL)
        .args([
            addr,
            "-w",
            "\n%{url_effective}",
            #[cfg(not(debug_assertions))]
            "-S",
        ])
        .output()
        .unwrap_or_else(|e| {
            _ = s.send(());
            quit!("curl: {}", e);
        });
    _ = s.send(());
    if !out.stderr.is_empty() {
        let err = String::from_utf8(out.stderr).unwrap_or_else(|e| e.to_string());
        quit!("Fetch {} failed - {err}", addr);
    }
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    let ll = s.rfind('\n').unwrap();
    _ = h.join();
    (s, ll)
}

///Parse photos in web url
fn parse(addr: &str) -> String {
    let (html, ll) = get_html(addr);
    let url_effective = &html[ll + 1..];
    let host = check_host(url_effective);
    let [mut img, mut next_sel, mut album, mut title] = host_info(host);
    let page = crabquery::Document::from(&html[..ll]);

    if img.is_none() {
        let cat = JSON
            .get_or_init(website)
            .as_object()
            .expect("`web.json` file parse error!")["Series"]
            .as_array()
            .expect("Parse `Series` in `web.json` key error!");
        if let Some(series) = cat.iter().find_map(|v| {
            let arr = v
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect::<Vec<_>>();

            if !page.select(arr[1]).is_empty() {
                Some(arr)
            } else {
                None
            }
        }) {
            tdbg!(series[0]);
            [img, next_sel, album, title] = host_info(series[2]);
        }
    }

    let sels = img.and_then(|i| i.split_once(SEP));
    let sel = sels.map(|(l, _)| l).or(img);

    let mut json_img = collections::HashSet::new();
    let mut html_img = vec![];
    let css_img = if img.is_none() {
        css_image(html.as_ref(), addr)
    } else {
        collections::HashSet::new()
    };

    if sel.is_some_and(|s| s.starts_with("json:")) {
        let kind = sel.unwrap().trim_start_matches("json:").trim();
        let name = sels.map(|(_, r)| r).unwrap().trim();
        let script = page.select("script");
        for s in script.iter().filter(|&s| s.text().is_some()) {
            let t = s.text().unwrap();
            let urls = t.split(name).skip(1);
            for u in urls {
                match kind {
                    "key" => {
                        let url = u
                            .split('"')
                            .nth(1)
                            .map(|u| u.replace(r"\u002F", "/"))
                            .unwrap();
                        json_img.insert(url);
                    }
                    "array" => {
                        u.split(['[', ']'])
                            .nth(1)
                            .unwrap()
                            .split(',')
                            .map(|s| s.trim().trim_matches('"').replace(r"\u002F", "/"))
                            .for_each(|url| {
                                json_img.insert(url);
                            });
                    }
                    _ => (),
                }
            }
        }
    } else {
        html_img = page.select(sel.unwrap_or("img"));
    }

    let attr = sel.map_or("src", |i| {
        i.split_whitespace()
            .next_back()
            .unwrap()
            .rsplit(['[', ']'])
            .nth(1)
            .unwrap_or("src")
    });

    let titles = page.select(if !json_img.is_empty() {
        "script"
    } else {
        "title"
    });

    let title = title.map_or_else(
        || {
            if !json_img.is_empty() {
                titles
                    .iter()
                    .find_map(|s| {
                        s.text()
                            .and_then(|t| t.split_once("metaKeywords").map(|kw| kw.1.to_owned()))
                    })
                    .unwrap()
                    .split('"')
                    .nth(1)
                    .unwrap()
                    .split(',')
                    .max_by_key(|&seg| seg.trim().len())
                    .unwrap()
                    .to_owned()
            } else {
                titles
                    .first()
                    .unwrap_or_else(|| {
                        quit!("Not a valid HTML page.");
                    })
                    .text()
                    .expect("NO title text.")
            }
        },
        |t| page.select(t)[0].text().unwrap(),
    );

    let mut t = title
        .rsplit(['/', '-', '_', '|', '–'])
        .skip(1)
        .max_by_key(|x| x.trim().len())
        .unwrap_or(title.as_str());

    let albums = album.map(|a| page.select(a));

    let has_album = album.is_some() && !albums.as_ref().unwrap().is_empty();
    let [albums_len, imgs_len, json_len] = [
        albums.as_ref().map_or(0, |a| a.len()),
        html_img.len() + css_img.len() + json_img.len(),
        json_img.len(),
    ];

    let term_title = link_text(t, addr);

    let name_count = |name: &[&str], count: &[usize]| -> String {
        name.iter()
            .zip(count)
            .filter_map(|(&n, &c)| {
                if c > 0 {
                    Some(format!(
                        "{n}{}",
                        if c == count.iter().sum::<usize>() {
                            String::default()
                        } else {
                            format!("({c})")
                        }
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(" + ")
    };

    let htj = name_count(
        ["HTML", "CSS", "JSON"].as_slice(),
        [html_img.len(), css_img.len(), json_len].as_slice(),
    );

    match (has_album, imgs_len > 0) {
        (true, true) => {
            pl!("Totally found <{albums_len}> 📸 and <{imgs_len}: {htj}> 🏞️  in 📄:{term_title}")
        }

        (true, false) => pl!("Totally found <{albums_len}> 📸 in 📄:{term_title}"),
        (false, true) => pl!("Totally found <{imgs_len}: {htj}> 🏞️  in 📄:{term_title}"),
        (false, false) => quit!("∅ 🏞️  found in 📄:{term_title}"),
    }

    t = if t.to_ascii_lowercase().contains(" page") || t.contains('页') {
        t[..t
            .to_ascii_lowercase()
            .rfind(" page")
            .or_else(|| t.rfind('第'))
            .unwrap_or(t.len())]
            .trim()
    } else {
        t[..t.rfind(['(', ',']).unwrap_or(t.len())].trim()
    };

    match (has_album, imgs_len > 0) {
        (_, true) => {
            let mut urls = collections::HashSet::new();
            let [mut empty, mut dup, mut embed] = [0usize; 3];

            for elm in html_img {
                let value = [
                    "data-src",
                    "data-lazy",
                    "data-lazy-src",
                    "data-original",
                    attr,
                ]
                .into_iter()
                .find_map(|a| elm.attr(a));
                let mut handle_embed = |s: String| {
                    if cfg!(feature = "embed") {
                        if !urls.insert(s) {
                            dup += 1;
                        }
                    } else {
                        embed += 1;
                    }
                };
                match value {
                    Some(val) => {
                        if attr == "style" {
                            if let Some(frag) = CSS.iter().find_map(|&s| val.trim().split_once(s)) {
                                let url = url_image(frag.1);
                                if let Some(u) = url {
                                    if u.starts_with("data:image/") {
                                        handle_embed(u);
                                    } else if u.is_empty() || !urls.insert(canonicalize(&u, addr)) {
                                        if !u.is_empty() {
                                            dup += 1;
                                        } else {
                                            empty += 1;
                                        }
                                    }
                                }
                            }
                        } else if val.starts_with("data:image/") {
                            handle_embed(val);
                        } else {
                            let url = if sel == img {
                                url_redirect_and_query_cleanup(&val)
                            } else {
                                val
                            };
                            // tdbg!(&url;);
                            if url.is_empty() || !urls.insert(canonicalize(&url, addr)) {
                                if !url.is_empty() {
                                    dup += 1;
                                } else {
                                    empty += 1;
                                }
                            }
                        }
                    }
                    None => {
                        empty += 1;
                    }
                }
            }
            let skip = empty + dup + embed;
            if skip > 0 {
                let edm = name_count(
                    ["Empty", "Duplicated", "Embed"].as_slice(),
                    [empty, dup, embed].as_slice(),
                );
                pl!("Skipped <{skip}: {edm}> 🏞️");
            }

            if let Some((l, r)) = sels {
                match r.trim().split_once("->") {
                    Some((mut old, mut new)) => {
                        old = old.trim();
                        new = new.trim();
                        let mut newurls = collections::HashSet::with_capacity(urls.len());
                        for mut u in urls {
                            if let Some(pos) = u.find(old) {
                                u.replace_range(pos..pos + old.len(), new);
                            }
                            newurls.insert(u);
                        }
                        urls = newurls;
                    }
                    _ if !l.starts_with("json:") && !urls.is_empty() => {
                        let mut curl = process::Command::new("curl");
                        for u in &urls {
                            curl.arg(u);
                        }
                        let o = curl
                            .args(CURL)
                            .args(["-Z", "--parallel-immediate"])
                            .output()
                            .unwrap();
                        let html = String::from_utf8_lossy(&o.stdout);
                        let page = crabquery::Document::from(html.as_ref());
                        let html_img = page.select(r);
                        urls.clear();
                        for e in html_img {
                            let src = e.attr("src").unwrap();
                            let title_alt = ["title", "alt"].iter().find_map(|a| {
                                e.attr(a).and_then(|x| {
                                    let attr = x.trim();
                                    if !attr.is_empty()
                                        && [".jpg", ".jpeg", ".png", ".webp", ".avif", ".bmp"]
                                            .iter()
                                            .any(|&ext| {
                                                attr.rfind('.').is_some_and(|dot| {
                                                    attr[dot..].eq_ignore_ascii_case(ext)
                                                })
                                            })
                                    {
                                        Some(x)
                                    } else {
                                        None
                                    }
                                })
                            });
                            urls.insert(title_alt.map_or_else(
                                || canonicalize(&src, addr),
                                |x| format!("{}{SEP}{x}", canonicalize(&src, addr)),
                            ));
                        }
                    }
                    _ => (),
                }
            }

            // tdbg!(&urls, &css_img, &json_img);
            download(t, urls.into_iter().chain(css_img).chain(json_img), host)
        }
        (true, false) => {
            let mut all = false;

            for (i, alb) in albums.unwrap().iter().enumerate() {
                let parse_album = || {
                    let href = alb.attr("href").unwrap_or_else(|| {
                        let mut p = alb.parent().unwrap();
                        let mut href = None;
                        let mut n = 2;
                        while n > 0 {
                            href = p.attr("href");
                            if href.is_some() {
                                break;
                            }
                            n -= 1;
                            if n > 0 {
                                p = p.parent().unwrap();
                            }
                        }

                        href.unwrap_or_else(|| {
                            p.select("a[href]")
                                .first()
                                .expect("NO album a[@href] attr found.")
                                .attr("href")
                                .unwrap()
                        })
                    });

                    if !href.is_empty() {
                        let album_url = canonicalize(&href, addr);
                        let mut next_page = parse(&album_url);
                        if cfg!(not(test)) {
                            while !next_page.is_empty() {
                                next_page = parse(&next_page);
                            }
                        }
                    }
                };

                if all {
                    parse_album();
                } else {
                    use io::*;

                    let stdin = stdin();
                    let mut stdout = stdout();

                    let t = ["title", "alt", "aria-label"]
                        .iter()
                        .find_map(|a| alb.attr(a))
                        .unwrap_or_else(|| {
                            alb.text().map_or_else(
                                || quit!("NO album title can be found."),
                                |x| {
                                    if x.trim().is_empty() {
                                        quit!("Album title text is empty.")
                                    } else {
                                        x
                                    }
                                },
                            )
                        });

                    _ = writeln!(
                        stdout,
                        "{B}Do you want to download Album <{U}{}/{albums_len}{_U}>: {G}{} ?{N}",
                        i + 1,
                        t.trim(),
                    );
                    _ = write!(
                        stdout,
                        "{MARK}{B}{Y}Y{u}es⏎{s}N{u}o{s}A{u}ll{s}C{u}ancel: {N}",
                        u = char::from_u32(0x332).unwrap(),
                        s = SEP,
                    );
                    _ = stdout.flush();

                    let mut input = String::new();
                    stdin.read_line(&mut input).unwrap_or_else(|e| {
                        quit!("{}", e);
                    });
                    input.make_ascii_lowercase();

                    match input.trim() {
                        "y" | "yes" | "" => parse_album(),
                        "n" | "no" => {
                            next_sel = None;
                            continue;
                        }
                        "a" | "all" => {
                            all = true;
                            parse_album()
                        }
                        _ => {
                            pl!("Canceled all albums download.");
                            next_sel = None;
                            break;
                        }
                    };
                }
            }
        }
        (false, false) => (),
    }

    next_sel.map_or_else(<_>::default, |n| {
        if n == "<script>" {
            if json_len == 0 {
                String::default()
            } else {
                let num = addr
                    .split_terminator('/')
                    .next_back()
                    .unwrap_or("")
                    .parse::<u8>()
                    .unwrap_or(1);
                let next_page = format!(
                    "{}/{}",
                    addr.trim_end_matches('/')
                        .trim_end_matches(&format!("/{num}")),
                    num + 1
                );
                tdbg!(next_page)
            }
        } else {
            check_next(n, addr, page)
        }
    })
}

///Canonicalize `img/next` link `url` in `addr`
fn canonicalize(url: &str, addr: &str) -> String {
    if url.is_empty() {
        return String::default();
    }
    let (scheme, path) = addr.split_once("://").unwrap_or(("http", addr));
    if !url.starts_with("http") {
        if url.starts_with("//") {
            format!("{scheme}:{url}")
        } else if url.starts_with('/') {
            format!(
                "{scheme}://{}{url}",
                &path[..path.find('/').unwrap_or(path.len())]
            )
        } else {
            format!(
                "{scheme}://{}/{url}",
                &path[..path.rfind('/').unwrap_or(path.len())]
            )
        }
    } else {
        url.to_owned()
    }
}

///replace os specific special/reversed chars in path name
fn sanitize_path(name: &str) -> String {
    #[cfg(target_os = "macos")]
    {
        name.replace("/", ":")
    }

    #[cfg(any(all(unix, not(target_os = "macos")), target_family = "wasm"))]
    {
        name.replace("/", "_")
    }

    #[cfg(target_family = "windows")]
    {
        name.chars()
            .map(|c| match c {
                '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                _ => c,
            })
            .collect()
    }
}
///Perform photo download operation
fn download(dir: &str, urls: impl Iterator<Item = String>, host: &str) {
    if cfg!(all(test, not(feature = "download"))) {
        return;
    }

    let slash2colon = sanitize_path(dir);
    let path = path::Path::new(&slash2colon);
    let create_dir = || {
        if !path.exists() {
            fs::create_dir(path).unwrap_or_else(|e| {
                quit!("Create Dir Error: {}", e);
            });
        }
    };

    let mut curl = process::Command::new("curl");
    curl.current_dir(path);

    #[cfg(feature = "infer")]
    let mut need_file_type_detection = vec![];

    #[cfg(not(feature = "infer"))]
    let mut no_ext = collections::HashMap::new();
    let mut no_ext_curl = process::Command::new("curl");
    no_ext_curl.args([
        "-Z",
        "--parallel-immediate",
        "-sIo",
        if cfg!(target_os = "windows") {
            "NUL"
        } else {
            "/dev/null"
        },
        "-w",
        "\n%{url} |-> %{content_type}\n",
    ]);
    static NAN: sync::OnceLock<percent_encoding::AsciiSet> = sync::OnceLock::new();
    let nan = NAN.get_or_init(|| {
        let remove = b".:/-_?=%";
        let mut ascii = percent_encoding::NON_ALPHANUMERIC.remove(remove[0]);
        for c in &remove[1..] {
            ascii = ascii.remove(*c);
        }
        ascii
    });
    for url in urls {
        if url.starts_with("data:image/") {
            #[cfg(feature = "embed")]
            {
                if let Ok(cur) = env::current_dir() {
                    create_dir();
                    _ = env::set_current_dir(path);

                    save_to_file(url.as_str());
                    _ = env::set_current_dir(cur);
                }
            }
            continue;
        }

        let lr = url.split_once(SEP);
        let u = lr.map_or(url.as_str(), |(l, _)| l);

        let mut name = u.rfind('/').map_or_else(
            || quit!("Invalid URL: {}", u),
            |slash| u[slash + 1..].trim_start_matches(['-', '_']),
        );

        name = &name[name.find("?url=").map_or(0, |u| u + 5)..];
        let name_no_query = &name[..name.find('?').unwrap_or(name.len())];
        let has_ext = name_no_query.rfind('.');
        #[cfg(not(feature = "infer"))]
        let mut name_ext = String::default();
        if has_ext.is_none() {
            #[cfg(feature = "infer")]
            {
                need_file_type_detection.push(name.to_owned());
            }

            #[cfg(not(feature = "infer"))]
            {
                lr.map_or_else(
                    || {
                        no_ext_curl.arg(&url);
                        no_ext.insert(url.clone(), name.to_owned());
                    },
                    |(_, file_name)| name_ext = file_name.into(),
                )
            }
        } else {
            name = name_no_query
        }

        #[cfg(not(feature = "infer"))]
        if no_ext.contains_key(&url) {
            continue;
        }
        #[cfg(not(feature = "infer"))]
        let file_name = if name_ext.is_empty() {
            name.trim_end_matches("!lrg")
        } else {
            name_ext.as_str()
        };
        #[cfg(feature = "infer")]
        let file_name = name;

        let enc_url = percent_encoding::utf8_percent_encode(u, nan).to_string();

        // tdbg!(&url, &enc_url);
        curl.args([&enc_url, "-o", file_name]);
    }

    // tdbg!(no_ext.keys());
    let opts = [
        "-e",
        &format!("https://{host}"),
        "-Z",
        "--parallel-immediate",
        "-C-",
    ];
    if curl.get_args().len() > 0 && cfg!(feature = "curl") {
        create_dir();
        let cmd = curl.args(CURL).args(opts);
        let _t = cmd.spawn();

        #[cfg(feature = "infer")]
        if !need_file_type_detection.is_empty() {
            _t.unwrap().wait().expect("curl download didn't run.");
            for f in need_file_type_detection {
                let file = path.join(&f);
                if file.exists() {
                    magic_number_type(file);
                }
            }
        }
    }

    #[cfg(not(feature = "infer"))]
    if !no_ext.is_empty() {
        create_dir();
        curl = process::Command::new("curl");
        curl.current_dir(path);

        no_ext_curl.output().map_or_else(
            |e| pl!("Query content-type info failed: {}", e),
            |o| {
                let res = String::from_utf8_lossy(&o.stdout);
                for (mut url, mut content_type) in res.lines().filter_map(|l| l.split_once("|->")) {
                    url = url.trim();
                    content_type = content_type.trim();
                    if let Some(ctx) = content_type.strip_prefix("image/") {
                        let ext = &ctx[..['+', ';', ',']
                            .iter()
                            .find_map(|&x| ctx.find(x))
                            .unwrap_or(ctx.len())];
                        let name = no_ext[url].as_str();
                        let name_ext = if !name.ends_with(ext) {
                            &format!("{name}.{ext}")
                        } else {
                            name
                        };
                        curl.args([url, "-o", name_ext]);
                    } else {
                        curl.args([
                            tdbg!(url),
                            "-o",
                            &format!("{}.ext!{content_type}", no_ext[url]),
                        ]);
                    }
                }
            },
        );
        if curl.get_args().len() > 0 {
            _ = curl.args(CURL).args(opts).spawn();
        }
    }
}

/// Infer file type through magic number
#[cfg(feature = "infer")]
fn magic_number_type(pb: path::PathBuf) {
    use io::*;

    let mut f =
        fs::File::open(&pb).unwrap_or_else(|e| quit!("Open file {} failed: {}", pb.display(), e));
    let mut buf = [0u8; 16];
    f.read_exact(&mut buf)
        .unwrap_or_else(|e| pl!("Read file {} magic number error: {}", pb.display(), e));

    let t = infer::get(&buf);
    // tdbg!(&t);
    fs::rename(
        &pb,
        pb.with_extension(t.map_or_else(
            || {
                let str = String::from_utf8_lossy(&buf);
                if str.contains("<svg") { "svg" } else { "" }
            },
            |ty| ty.extension(),
        )),
    )
    .unwrap_or_else(|e| pl!("Rename {} failed: {}", pb.display(), e));
}

/// Check `next` selector link page info
fn check_next(next: &str, cur: &str, page: crabquery::Document) -> String {
    let mut next_link: String;
    let ns = next.split_once(SEP);
    let nxt = ns.map_or(next, |(l, _)| l);
    let attr = nxt
        .split_whitespace()
        .next_back()
        .unwrap()
        .rsplit(['[', ']'])
        .nth(1)
        .unwrap_or("href");
    let set_next = |tags: &[crabquery::Element]| -> String {
        let tag = tags.iter().find(|e| {
            e.tag().unwrap() == "a"
                || e.children()
                    .first()
                    .is_some_and(|c| c.tag().unwrap() == "a")
        });
        tag.map_or(String::default(), |e| {
            if e.text().is_none_or(|t| t.trim().is_empty()) && e.children().is_empty() {
                <_>::default()
            } else {
                e.attr(attr)
                    .or_else(|| e.children().first().and_then(|x| x.attr(attr)))
                    .unwrap()
            }
        })
    };
    let mut nexts = page.select(nxt);
    nexts.sort_by_cached_key(|x| x.attr(attr));
    nexts.dedup_by_key(|x| x.attr(attr));

    if nexts.is_empty() {
        next_link = String::default();
        tdbg!("NO next page <element> found with selector: {nxt}");
    } else if nexts.len() == 1 {
        let element = &nexts[0];
        if element.tag().unwrap() == "span" || element.attr(attr).is_none() {
            let items = element.parent().unwrap().children();
            let tags = items.split(|e| element.eql(e)).next_back().unwrap();
            next_link = set_next(tags);
        } else if element.tag().unwrap() == "i" {
            next_link = element.parent().unwrap().attr(attr).unwrap();
        } else {
            next_link = element.attr(attr).unwrap();
        }
    } else {
        let last2 = nexts[nexts.len() - 2..].iter().rfind(|&n| {
            let mut t = n.text();
            if t.is_some() && t.as_deref().unwrap().trim().is_empty() {
                t.take();
            }
            let next_下 = |mut t: String| {
                t.make_ascii_lowercase();
                t.contains('下') || t.contains("next")
            };
            match t {
                Some(text) => next_下(text) || (n.attr("target").is_some()),
                None => {
                    t = n.attr("title");
                    match t {
                        Some(title) => next_下(title),
                        None => {
                            let span = n.select("span.currenttext");
                            if span.is_empty() {
                                return false;
                            }
                            t = span[0].text();
                            match t {
                                Some(text) => next_下(text),
                                None => false,
                            }
                        }
                    }
                }
            }
        });
        next_link = match last2 {
            Some(v) => v.attr(attr).unwrap_or(String::default()),
            None => {
                let pos = nexts.iter().rposition(|e| {
                    e.attr(attr).is_some_and(|h| {
                        cur.trim().ends_with(h.trim())
                            || h.trim() == "#"
                            || ["/1", "?page=1", "/page/1"].iter().any(|suffix| {
                                format!("{}{suffix}", cur.trim_end_matches('/')).ends_with(h.trim())
                            })
                    })
                });
                match pos {
                    Some(p) => {
                        if p < nexts.len() - 1 {
                            nexts[p + 1].attr(attr).unwrap()
                        } else {
                            String::default()
                        }
                    }
                    None => String::default(),
                }
            }
        };
    }
    // if !next.is_empty() && !next[next.rfind('/').unwrap()..].contains(['_', '-', '?']) {
    //     next = String::default();
    // }

    if cur.trim().ends_with(&next_link)
        || next_link.trim() == "#"
        || next_link.trim() == "javascript:;"
        || next_link.trim() == "/"
    {
        next_link = String::default();
    }
    if !next_link.is_empty() {
        next_link = ns.map_or_else(
            || canonicalize(&next_link, cur),
            |(_, r)| {
                let count = r.matches('/').count();
                format!(
                    "{}{r}",
                    if cur.contains(r.split("{}").next().unwrap()) {
                        cur.trim_end_matches('/')
                            .rsplitn(count, '/')
                            .last()
                            .unwrap()
                    } else {
                        cur.trim_end_matches('/')
                    }
                )
                .replace("{}", &next_link)
            },
        );
    }

    tdbg!(next_link)
}

///WebSites `Json` config data
fn website() -> serde_json::Value {
    serde_json::from_str(include_str!("web.json")).unwrap_or_else(|e| {
        quit!("Read `web.json` failed: {}", e);
    })
}

///Save inline/embed `data:image/..+..;base64,...` or `base64/url-escaped` content to file.
#[cfg(feature = "embed")]
fn save_to_file(data: &str) {
    if cfg!(not(feature = "embed")) {
        return;
    }

    let ctx = &data["data:image/".len()..data.find(',').unwrap()];
    let ext = &ctx[..['+', ';']
        .iter()
        .find_map(|&x| ctx.find(x))
        .unwrap_or(ctx.len())];

    let generate_name = || -> String {
        let t = format!("{:?}", time::Instant::now());
        let name = &t[t.rfind(':').unwrap() + 2..t.len() - 2];
        format!("{name}.{ext}")
    };
    let mut full_name = generate_name();
    //Prevent overwriting other images with the same file name.
    while path::Path::new(&full_name).exists() {
        full_name = generate_name();
    }

    let content = &data[data.find(',').unwrap() + 1..];
    use base64::*;
    {
        if ctx.contains(";base64") {
            let mut buf = vec![0; content.len()];
            let size = engine::general_purpose::STANDARD
                .decode_slice(content, &mut buf)
                .unwrap_or_else(|e| quit!("{e}"));
            buf.truncate(size);
            fs::write(&full_name, buf)
        } else {
            fs::write(
                &full_name,
                percent_encoding::percent_decode_str(content)
                    .decode_utf8_lossy()
                    .as_ref(),
            )
        }
    }
    .unwrap_or_else(|e| quit!("Write {ctx} to file {full_name} failed: {}", e));
}

///Show `circle` progress indicator
fn circle_indicator(r: sync::mpsc::Receiver<()>) {
    use io::*;
    use sync::mpsc::*;

    let chars = ['◯', '◔', '◑', '◕', '●'];
    // let chars = ["◯", "◔.", "◑..", "◕...", "●...."];
    let mut o = stdout().lock();
    let t = time::Instant::now();
    'l: loop {
        for char in chars {
            let secs = t.elapsed().as_secs();
            print!(
                "{BEG}{char}...{}",
                if secs > 0 {
                    format!("{secs:>2}s")
                } else {
                    String::default()
                }
            );
            _ = o.flush();
            match r.try_recv() {
                Err(TryRecvError::Empty) => (),
                _ => break 'l,
            }
            thread::yield_now();
            thread::sleep(time::Duration::from_millis(200));
        }
    }
    print!("{CL}{BEG}");
    _ = o.flush();
}

///cleanup url
fn url_redirect_and_query_cleanup(url: &str) -> String {
    use percent_encoding::*;
    let dec_url = percent_decode_str(url).decode_utf8_lossy();
    let mut cleanup = &dec_url[dec_url.rfind("?url=").map_or(0, |p| p + 5)..];
    cleanup = &cleanup[..cleanup
        .find('?')
        .and_then(|q| cleanup[q..].find('&').map(|a| a + q))
        .or_else(|| {
            cleanup.rfind('/').and_then(|slash| {
                cleanup[slash..].rfind('.').and_then(|dot| {
                    cleanup[slash + dot..]
                        .find('&')
                        .map(|amp| amp + dot + slash)
                })
            })
        })
        .unwrap_or(cleanup.len())];
    cleanup.into()
}

///Parse inline `url(),image()`
fn url_image(content: &str) -> Option<String> {
    if let Some(rp) = content.find(')') {
        let mut url = &content[..rp];
        _ = ["ltr ", "rtl "].map(|x| url = url.trim_start_matches(x));
        url = url.trim_matches(['\'', '"']).trim();
        _ = ["&#39;", "&apos;", "&#34;", "&quot;"]
            .map(|x| url = url.trim_start_matches(x).trim_end_matches(x).trim());
        if url.starts_with("data:image/") {
            return Some(url.into());
        }
        let dec = url_redirect_and_query_cleanup(url);
        url = dec.as_str();
        url = &url[..url.rfind("#xywh").unwrap_or(url.len())];
        if url.is_empty()
            || url == "undefined"
            || url.starts_with(['{', '$'])
            || url.contains('#')
            || [
                ".jpg", ".jpeg", ".jxl", ".png", ".webp", ".bmp", ".tif", ".tiff", ".ico", ".gif",
                ".svg", ".svgz", ".avif", ".heif", ".heic", ".jp2", ".j2k", ".jpx",
            ]
            .iter()
            .all(|&ext| !url.to_ascii_lowercase().ends_with(ext))
        {
            None
        } else {
            Some(url.trim().into())
        }
    } else {
        None
    }
}

///Get `page` css style `url(),image(),image-set()`
fn css_image(html: &str, addr: &str) -> collections::HashSet<String> {
    let mut images = collections::HashSet::new();
    _ = CSS.map(|s| {
        let segments = html.split(s);
        if s == "image-set(" {
            for seg in segments.skip(1) {
                images = images
                    .union(&css_image(seg, addr))
                    .map(Into::into)
                    .collect();
            }
        } else {
            for seg in segments.skip(1) {
                if let Some(u) = url_image(seg) {
                    if u.starts_with("data:image/") {
                        if cfg!(feature = "embed") {
                            images.insert(u);
                        }
                    } else {
                        images.insert(canonicalize(&u, addr));
                    }
                }
            }
        }
    });
    images
}

///Linkable Text based upon terminal type
fn link_text(text: &str, addr: &str) -> String {
    if env::var("TERM").is_ok_and(|o| {
        ["term", "vt", "crt", "pty", "emu", "virt", "onsole"]
            .iter()
            .any(|x| o.contains(x))
    }) {
        format!("{G} \x1b]8;;{addr}\x1b\\{text}\x1b]8;;\x1b\\")
    } else {
        format!("{G} {text}")
    }
}

#[cfg(test)]
mod img {

    use super::*;

    #[inline]
    fn arg(default: &str) -> String {
        let arg = env::args().nth(4);
        arg.unwrap_or(String::from(default))
    }

    #[test]
    fn html() {
        let html = get_html(&arg("mmm.red"));
        dbg!(&html);
    }

    #[test]
    fn htmlq() {
        let addr = arg("https://www.hotgirlpix.com/");
        let host = check_host(&addr);
        let (html, ll) = get_html(&addr);
        let [img, _, album, _] = host_info(host);
        use process::*;

        let hq = |sel: &str| {
            let cmd = Command::new("htmlq")
                .arg(sel)
                .stdin(Stdio::piped())
                //.stdout(Stdio::piped())
                .spawn()
                .expect("Execute htmlq failed.");
            let mut stdin = cmd.stdin.as_ref().expect("Failed to open stdin.");
            use io::*;
            stdin
                .write_all(html[..ll].as_ref())
                .expect("Failed to write stdin.");
            if let Ok(o) = cmd.wait_with_output() {
                if !o.stdout.is_empty() {
                    println!("Totally found {} <img>", o.stdout.lines().count());
                }
            }
        };

        let i = img.unwrap_or("img[src]");
        pl!("{MARK} Image Selector: {HL} {i} ");
        hq(i);

        if let Some(a) = album {
            pl!("{MARK} Album Selector: {HL} {a} ");
            hq(a)
        }
    }

    #[test]
    fn mut_val() {
        let var = 123;
        mutv!(var, 100 * 2 + 22);
        tdbg!(var);
    }

    // fn(..) -> Pin<Box<impl/dyn Future<Output = Something> + '_>>

    #[test]
    fn run() {
        // https://bisipic.online/portal.php?page=2

        if let Some(arg) = env::args().nth(4) {
            parse(&arg);
        } else {
            [
                "https://xiutaku.com",
                "https://meitu9.com/",
                "https://bisipic.online",
            ]
            .into_iter()
            .for_each(|u| {
                pl!("Parsing... {}", u);
                parse(u);
                pause();
            });
        }
    }

    #[test]
    fn union_match() {
        //fields superimpose over one another
        union IntOrFloat {
            i: i32,
            f: f32,
        }

        let u = IntOrFloat { f: 1.0 };

        unsafe {
            match u {
                IntOrFloat { i: 10 } => println!("Found exactly ten!"),
                // Matching the field `f` provides an `f32`.
                IntOrFloat { f } => println!("Found f = {f} !"),
            }
        }
    }

    #[test]
    fn css_img() {
        let addr = arg("autodesk.com");
        let (html, ll) = get_html(&addr);
        let r = css_image(&html[..ll], &addr);
        tdbg!(&r, r.len());
    }

    #[test]
    fn progress() {
        use sync::mpsc::*;
        let (s, r) = channel();
        thread::spawn(|| {
            circle_indicator(r);
        });
        thread::yield_now();
        thread::sleep(time::Duration::from_secs(5));
        s.send(()).unwrap_or_else(|e| pl!("send error: {}", e));
    }

    #[test]
    fn sanity_check_json() {
        let mut sites = collections::HashSet::new();
        let mut dup_site = vec![];
        let mut img_sel = collections::HashMap::new();

        JSON.get_or_init(website)
            .as_object()
            .expect("`web.json` file parse error!")["Sites"]
            .as_array()
            .expect("Parse `Sites` in `web.json` key error!")
            .iter()
            .for_each(|s| {
                if let Some(v) = s["Site"].as_str() {
                    v.split_terminator(',').for_each(|domain| {
                        if !sites.insert(domain.trim()) {
                            dup_site.push(domain);
                        }
                    });
                    let img = s["Img"].as_str().unwrap().trim();
                    if let Some(mut old) = img_sel.insert(img, vec![v]) {
                        old.push(v);
                        img_sel.insert(img, old);
                    }
                }
            });

        pl!(
            "Todally find {} web sites, with duplicated {} sites.",
            sites.len(),
            dup_site.len()
        );
        if !dup_site.is_empty() {
            dbg!(&dup_site);
        }
        assert!(dup_site.is_empty());

        let mut dup_sel = img_sel
            .keys()
            .filter_map(|k| {
                if img_sel[*k].len() > 1 {
                    Some((*k, img_sel[*k].len()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        pl!(
            "Todally find {} Img selectors, with duplicated {} selectors.",
            img_sel.len(),
            dup_sel.len()
        );

        if !dup_sel.is_empty() {
            dup_sel.sort_unstable_by_key(|x| cmp::Reverse(x.1));
            for (sel, count) in dup_sel {
                pl!("({},{})", sel, count);
            }
        }
    }

    #[test]
    fn file_type() {
        let dir = env::current_dir().unwrap();
        let _f = dir.join("demo.file");
        #[cfg(feature = "infer")]
        magic_number_type(_f);
    }

    #[cfg(feature = "embed")]
    #[test]
    fn embed() {
        let data = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNgYAAAAAMAASsJTYQAAAAASUVORK5CYII=";
        #[cfg(feature = "embed")]
        save_to_file(data);
    }

    #[test]
    fn batch() {
        if cfg!(not(feature = "download")) {
            return;
        }
        let mut skip3 = env::args().skip(3);
        let addr = skip3
            .nth(1)
            .unwrap_or("https://girldreamy.com/category/china/xiuren/page/30".into());
        let count = skip3
            .nth(2)
            .unwrap_or("1".into())
            .parse::<u16>()
            .unwrap_or_else(|x| {
                println!("Invalid batch count: {x}");
                0
            });
        tdbg!(&addr, count);

        let num = &addr[addr.rfind('/').unwrap() + 1..]
            .parse::<u16>()
            .expect("Parse page number failed.");

        (0..count).map(|i| num - i).for_each(|p| {
            let mut idx = format!("{}{p}", &addr[..=addr.rfind('/').unwrap()]);
            tdbg!(&idx);
            idx = parse(&idx);
            while !idx.is_empty() {
                idx = parse(&idx);
            }
        });
    }
}
