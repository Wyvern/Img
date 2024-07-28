#![cfg_attr(not(debug_assertions), no_main)]
mod util;
use {std::*, util::*};

static CSS: [&str; 3] = ["url(", "image(", "image-set("];
static JSON: sync::OnceLock<serde_json::Value> = sync::OnceLock::new();
static CURL: [&str; 7] = [
    "--compressed",
    "-k",
    "-A",
    "Mozilla/5.0 Firefox/Edge/Chrome",
    "--tcp-fastopen",
    "--tcp-nodelay",
    //"--mptcp",
    if cfg!(debug_assertions) {
        "-fsSL"
    } else {
        "-fsL"
    },
];

fn check_args() -> String {
    if env::args().len() > if cfg!(test) { 2 + 3 } else { 2 } {
        quit!("Too many arguments.\nUsage: {}", "Img <url>");
    }

    if cfg!(test) {
        env::args().skip(3).nth(1)
    } else {
        env::args().nth(1)
    }
    .unwrap_or_else(|| {
        quit!("Please input <url> argument.");
    })
}

#[cfg_attr(not(debug_assertions), no_mangle)]
fn main() {
    let arg = check_args();
    let mut next_page = parse(&arg);

    if cfg!(not(test)) {
        while !next_page.is_empty() {
            next_page = parse(&next_page);
        }
    }
}

///Get `scheme` and `host` info from valid url string
fn check_host(addr: &str) -> [&str; 2] {
    let split = addr.split_once("://").unwrap_or(("http", addr));

    let scheme = split.0;
    if scheme.is_empty()
        || !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
    {
        quit!("{}: Invalid {U}http(s) protocol.", scheme);
    }
    let rest = split.1;
    let host = &rest[..rest.find('/').unwrap_or(rest.len())];
    if host.is_empty() || !host.contains('.') {
        quit!("{}: Invalid host info.", host);
    }
    [scheme, host]
}

///Get `host` info and Generate `img/next/album` selector data
fn host_info(host: &str) -> [Option<&str>; 3] {
    let site = JSON
        .get_or_init(website)
        .as_array()
        .expect("Json file parse error.")
        .iter()
        .find(|&s| {
            s["Site"].as_str().map_or(false, |s| {
                s.split_terminator(',')
                    .any(|s| host.trim_end().ends_with(s.trim()))
            })
        });
    site.map_or([None; 3], |s| {
        ["Img", "Next", "Album"].map(|key| s[key].as_str())
    })
}

///Fetch web page generate html content
fn get_html(addr: &str) -> (String, [Option<&str>; 3], [&str; 2]) {
    let scheme_host @ [_, host] = check_host(addr);
    let host_info = host_info(host);
    use sync::mpsc::*;
    let (s, r) = channel();
    let _ = io::stdout().lock();
    thread::spawn(|| {
        circle_indicator(r);
    });
    let out = process::Command::new("curl")
        .args(CURL)
        .args([
            addr,
            #[cfg(not(debug_assertions))]
            "-S",
        ])
        .output()
        .unwrap_or_else(|e| {
            let _ = s.send(());
            quit!("curl: {}", e);
        });
    let _ = s.send(());
    if out.stdout.is_empty() {
        let err = String::from_utf8(out.stderr).unwrap_or_else(|e| e.to_string());
        quit!("Fetch {} failed - {err}", addr);
    }
    let res = String::from_utf8_lossy(&out.stdout);
    (res.into_owned(), host_info, scheme_host)
}

///Parse photos in web url
fn parse(addr: &str) -> String {
    let (html, [img, mut next_sel, album], [scheme, host]) = get_html(addr);
    let css_img = if img.is_none() {
        css_image(&html, scheme, addr)
    } else {
        collections::HashSet::new()
    };
    let sels = img.and_then(|i| i.split_once(" | "));
    let sel = sels.map(|(l, _)| l).or(img);
    let page = crabquery::Document::from(html);
    let html_img = page.select(sel.unwrap_or("img"));
    let attr = sel.map_or("src", |i| match ['[', ']'].map(|x| i.rfind(x)) {
        [Some(lbrace), Some(rbrace)] if i.trim_end().ends_with(']') => &i[lbrace + 1..rbrace],
        _ => "src",
    });

    let titles = page.select("title");
    let title = titles
        .first()
        .unwrap_or_else(|| {
            quit!("Not a valid HTML page.");
        })
        .text()
        .expect("NO title text.");
    let mut t = title.trim();

    t = t
        .split(['/', '-', '_', '|', '–'])
        .max_by_key(|x| x.len())
        .unwrap()
        .trim();

    let albums = album.map(|a| page.select(a));

    let has_album = album.is_some() && !albums.as_ref().unwrap().is_empty();
    let [albums_len, imgs_len, html, css] = [
        albums.as_ref().map_or(0, |a| a.len()),
        html_img.len() + css_img.len(),
        html_img.len(),
        css_img.len(),
    ];

    let term_title = if terminal_emulator() {
        format!("{G} \x1b]8;;{addr}\x1b\\{t}\x1b]8;;\x1b\\")
    } else {
        format!("{G} {t}")
    };

    let htmlcss = if html > 0 && css > 0 {
        format!(": HTML({html}) + CSS({css})")
    } else if html > 0 {
        format!(": HTML({html})")
    } else if css > 0 {
        format!(": CSS({css})")
    } else {
        String::new()
    };
    match (has_album, imgs_len > 0) {
        (true, true) => {
            pl!("Totally found <{albums_len}> 📸 and <{imgs_len}{htmlcss}> 🏞️  in 📄:{term_title}")
        }
        (true, false) => pl!("Totally found <{albums_len}> 📸 in 📄:{term_title}"),
        (false, true) => pl!("Totally found <{imgs_len}{htmlcss}> 🏞️  in 📄:{term_title}"),
        (false, false) => quit!("∅ 🏞️  found in 📄:{term_title}"),
    }

    t = if t.contains("page") || t.contains('页') {
        t[..t.rfind("page").or_else(|| t.rfind('第')).unwrap_or(t.len())].trim()
    } else {
        t[..t.rfind(['(', ',']).unwrap_or(t.len())].trim()
    };

    match (has_album, imgs_len > 0) {
        (_, true) => {
            let mut urls = collections::HashSet::new();
            let [mut empty_dup, mut embed] = [0u16; 2];

            for elm in html_img {
                let value = ["data-src", "data-lazy", "data-lazy-src", attr]
                    .iter()
                    .find_map(|&a| elm.attr(a));

                match value {
                    Some(val) => {
                        if attr == "style" {
                            if let Some(frag) = CSS.iter().find_map(|&s| val.trim().split_once(s)) {
                                let url = url_image(frag.1);
                                if let Some(u) = url {
                                    if u.starts_with("data:image/") {
                                        if cfg!(feature = "embed") {
                                            if !urls.insert(u) {
                                                empty_dup += 1;
                                            }
                                        } else {
                                            embed += 1;
                                        }
                                    } else if !urls.insert(canonicalize(u, scheme, addr)) {
                                        empty_dup += 1;
                                    }
                                }
                            }
                        } else if val.starts_with("data:image/") {
                            if cfg!(feature = "embed") {
                                if !urls.insert(val) {
                                    empty_dup += 1;
                                }
                            } else {
                                embed += 1;
                            }
                        } else {
                            let url = if sel == img {
                                url_redirect_and_query_cleanup(&val)
                            } else {
                                val
                            };

                            // tdbg!(&url);
                            if url.is_empty() || !urls.insert(canonicalize(url, scheme, addr)) {
                                empty_dup += 1;
                            }
                        }
                    }
                    None => {
                        empty_dup += 1;
                    }
                }
            }

            if empty_dup > 0 && embed > 0 {
                let skip = empty_dup + embed;
                pl!("Skipped <{skip}> Empty/Duplicated/Embed 🏞️");
            } else if empty_dup > 0 {
                pl!("Skipped <{empty_dup}> Empty/Duplicated 🏞️");
            } else if embed > 0 {
                pl!("Skipped <{embed}> Embed 🏞️");
            }

            if let Some((_, r)) = sels {
                let mut curl = process::Command::new("curl");
                curl.arg("-Z");
                for u in &urls {
                    curl.arg(u);
                }
                let o = curl
                    .args(CURL)
                    .arg("--parallel-immediate")
                    .output()
                    .unwrap();
                let html = String::from_utf8_lossy(&o.stdout).into_owned();
                let page = crabquery::Document::from(html);
                let html_img = page.select(r);
                urls.clear();
                for e in html_img {
                    let src = e.attr("src").unwrap();
                    let title_alt = ["title", "alt"].iter().find_map(|a| {
                        e.attr(a).and_then(|x| {
                            if !x.is_empty()
                                && [".jpg", ".jpeg", ".png", ".webp"]
                                    .iter()
                                    .any(|&ext| x.trim_end().ends_with(ext))
                            {
                                Some(x)
                            } else {
                                None
                            }
                        })
                    });

                    if title_alt.is_some() {
                        urls.insert(format!(
                            "{}|{}",
                            canonicalize(src, scheme, addr),
                            title_alt.unwrap()
                        ));
                    } else {
                        urls.insert(canonicalize(src, scheme, addr));
                    }
                }
            }
            // tdbg!(&urls, &css_img);
            download(t, urls.into_iter().chain(css_img), host)
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
                        let album_url = canonicalize(href, scheme, addr);
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
                                        quit!("[Album title is empty]")
                                    } else {
                                        x
                                    }
                                },
                            )
                        });

                    let _ = writeln!(
                        stdout,
                        "{B}Do you want to download Album <{U}{}/{albums_len}{_U}>: {G}{} ?{N}",
                        i + 1,
                        t.trim(),
                    );
                    let _ = write!(
                        stdout,
                        "{MARK}{B}{Y}Y{u}es⏎{s}N{u}o{s}A{u}ll{s}C{u}ancel: {N}",
                        u = char::from_u32(0x332).unwrap(),
                        s = " | ",
                    );
                    let _ = stdout.flush();

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

    next_sel.map_or_else(<_>::default, |n| check_next(page.select(n), scheme, addr))
}

///Canonicalize `img/next` link `url` in `addr`
fn canonicalize(url: String, scheme: &str, addr: &str) -> String {
    if url.is_empty() {
        return url;
    }

    if !url.starts_with("http") {
        if url.starts_with("//") {
            format!("{scheme}:{url}")
        } else if url.starts_with('/') {
            let path = addr.split_once("://").unwrap_or((scheme, addr)).1;
            format!(
                "{scheme}://{}{url}",
                &path[..path.find('/').unwrap_or(path.len())]
            )
        } else {
            let path = addr.split_once("://").unwrap_or((scheme, addr)).1;
            format!(
                "{scheme}://{}/{url}",
                &path[..path.rfind('/').unwrap_or(path.len())]
            )
        }
    } else {
        url
    }
}

///Perform photo download operation
fn download(dir: &str, urls: impl Iterator<Item = String>, host: &str) {
    if cfg!(all(test, not(feature = "download"))) {
        return;
    }
    let slash2colon = dir.replace('/', ":");
    let path = path::Path::new(&slash2colon);
    let create_dir = || {
        if !path.exists() {
            fs::create_dir(path).unwrap_or_else(|e| {
                quit!("Create Dir Error: {}", e);
            });
        }
    };

    let mut curl = process::Command::new("curl");
    curl.current_dir(path).arg("-Z");

    #[cfg(feature = "infer")]
    let mut need_file_type_detection = vec![];
    #[cfg(not(feature = "infer"))]
    use sync::mpsc::*;
    #[cfg(not(feature = "infer"))]
    let (s, r) = channel();
    #[cfg(not(feature = "infer"))]
    let sender = sync::Arc::new(s);
    #[cfg(not(feature = "infer"))]
    let mut no_ext = collections::HashMap::new();

    for url in urls {
        if url.starts_with("data:image/") {
            #[cfg(feature = "embed")]
            {
                if let Ok(cur) = env::current_dir() {
                    create_dir();
                    let _ = env::set_current_dir(path);

                    save_to_file(url.as_str());
                    let _ = env::set_current_dir(cur);
                }
            }
            continue;
        }

        let lr = url.rsplit_once('|');
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
                        no_ext.insert(url.clone(), String::default());
                        let (u, n, s) = (url.clone(), name.to_owned(), sender.clone());
                        thread::spawn(|| {
                            content_header_info(u, n, s);
                        });
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
            name
        } else {
            name_ext.as_str()
        };
        #[cfg(feature = "infer")]
        let file_name = name;

        if !path.join(file_name).exists() {
            static NAN: sync::OnceLock<percent_encoding::AsciiSet> = sync::OnceLock::new();
            let enc_url = percent_encoding::utf8_percent_encode(
                u,
                NAN.get_or_init(|| {
                    percent_encoding::NON_ALPHANUMERIC
                        .remove(b':')
                        .remove(b'/')
                        .remove(b'.')
                        .remove(b'-')
                        .remove(b'_')
                        .remove(b'?')
                        .remove(b'=')
                }),
            )
            .to_string();

            // tdbg!(&url, &enc_url);
            curl.args([&enc_url, "-o", file_name]);
        }
    }

    // tdbg!(no_ext.keys());

    if curl.get_args().len() > 1 && cfg!(feature = "curl") {
        create_dir();
        let cmd = curl
            .args(CURL)
            .args(["-e", &format!("https://{host}"), "--parallel-immediate"]);
        #[cfg(not(feature = "infer"))]
        let _ = cmd.spawn();

        #[cfg(feature = "infer")]
        if !need_file_type_detection.is_empty() {
            let _ = cmd.output();
            let sync = true;
            if sync {
                for f in need_file_type_detection {
                    let file = path.join(&f);
                    if file.exists() {
                        magic_number_type(file);
                    }
                }
            } else {
                let (p, h) = (path.to_owned(), host.to_owned());
                thread::spawn(move || {
                    let _ = curl
                        .args(CURL)
                        .args(["-e", &h, "--parallel-immediate"])
                        .output();
                    for f in need_file_type_detection {
                        let file = p.join(&f);
                        if file.exists() {
                            magic_number_type(file);
                        }
                    }
                });
            }
        }
    }

    #[cfg(not(feature = "infer"))]
    if !no_ext.is_empty() {
        create_dir();
        curl = process::Command::new("curl");
        curl.current_dir(path).arg("-Z");
        while no_ext.values().any(|v| v.is_empty()) {
            match r.recv() {
                Ok((url, name_ext)) => {
                    curl.args([url.as_str(), "-o", name_ext.as_str()]);
                    no_ext.insert(url, name_ext);
                }
                Err(e) => {
                    pl!("receive error: {}", e);
                }
            }
        }
        let _ = curl
            .args(CURL)
            .args(["-e", &format!("https://{host}"), "--parallel-immediate"])
            .spawn();
    }

    // thread::sleep(time::Duration::from_secs(3));
}

/// Get `url` content header info to generate full `name.ext`
fn content_header_info(
    url: String,
    name: String,
    s: sync::Arc<sync::mpsc::Sender<(String, String)>>,
) {
    let mut name_ext = String::default();
    // tdbg!(&url);
    process::Command::new("curl")
        .args(["-J", "-w", "%{content_type}"])
        .args(CURL)
        .arg(&url)
        .output()
        .map_or_else(
            |e| pl!("Get {} content type info failed: {}", &url, e),
            |o| {
                let header = String::from_utf8_lossy(&o.stdout);
                if let Some(l) = header.lines().last() {
                    if let Some((_, ctx)) = l.rsplit_once("image/") {
                        let ext = &ctx[..['+', ';', ',']
                            .iter()
                            .find_map(|&x| ctx.find(x))
                            .unwrap_or(ctx.len())];
                        name_ext = if !name.ends_with(format!(".{ext}").as_str()) {
                            format!("{name}.{ext}")
                        } else {
                            name.clone()
                        }
                    }
                }
            },
        );
    if name_ext.is_empty() {
        pl!("Get `{}` with `{}` extension error,", url, name);
        name_ext = format!("{name}.ext!")
    }
    s.send((url, name_ext))
        .unwrap_or_else(|e| pl!("send error: {}", e));
}

/// Infer file type through magic number
#[cfg(feature = "infer")]
fn magic_number_type(pb: path::PathBuf) {
    use io::*;

    let mut f = fs::File::open(&pb).unwrap_or_else(|e| quit!("{e} : {}", pb.display()));
    let mut buf = [0u8; 16];
    f.read_exact(&mut buf)
        .unwrap_or_else(|e| pl!("Read file magic number error: {}", e));

    let t = infer::get(&buf);
    // tdbg!(&t);
    fs::rename(
        &pb,
        pb.with_extension(t.map_or_else(
            || {
                let str = String::from_utf8_lossy(&buf);
                if str.contains("<svg") {
                    "svg"
                } else {
                    ""
                }
            },
            |ty| ty.extension(),
        )),
    )
    .unwrap_or_else(|e| pl!("Rename {} failed: {}", pb.display(), e));
}

/// Check `next` selector link page info
fn check_next(nexts: Vec<crabquery::Element>, scheme: &str, cur: &str) -> String {
    let mut next_link: String;
    let splitter = |tag: &crabquery::Element| {
        tag.attr("class")
            .is_some_and(|c| ["cur", "now", "active"].iter().any(|cls| c.contains(cls)))
    };
    if nexts.is_empty() {
        next_link = String::default();
        //println!("NO next page <element> found.")
    } else if nexts.len() == 1 {
        let element = &nexts[0];
        if element.tag().unwrap() == "span" || element.attr("href").is_none() {
            let items = element.parent().unwrap().children();
            let mut tags = items.split(|e| {
                (e.tag().unwrap() == "span" || e.attr("href").is_none())
                    && (splitter(e)
                        || items.iter().filter(|x| x.tag().unwrap() == "span").count() == 1)
            });
            let a = tags
                .next_back()
                .unwrap()
                .iter()
                .filter(|e| {
                    e.tag().unwrap() == "a"
                        || e.children()
                            .first()
                            .is_some_and(|c| c.tag().unwrap() == "a")
                })
                .collect::<Vec<_>>();

            next_link = a.first().map_or(String::default(), |f| {
                if f.text().map_or(true, |t| t.trim().is_empty()) && f.children().is_empty() {
                    String::default()
                } else {
                    f.attr("href")
                        .or_else(|| f.children().first().and_then(|x| x.attr("href")))
                        .unwrap()
                }
            });
        } else if element.tag().unwrap() == "i" {
            next_link = element.parent().unwrap().attr("href").unwrap();
        } else {
            next_link = element.attr("href").unwrap();
        }
    } else {
        let element = &nexts[0];
        if element.tag().unwrap() == "div" && nexts.len() == 2 {
            let tags = element.children();
            let mut rest = tags.split(|tag| {
                tag.children()
                    .first()
                    .map_or_else(|| tag.tag().unwrap() == "span" || splitter(tag), splitter)
            });
            let s = rest.next_back().unwrap();
            next_link = s.first().map_or(String::default(), |f| {
                f.children()
                    .first()
                    .map_or_else(|| f.attr("href").unwrap(), |ff| ff.attr("href").unwrap())
            });
        } else {
            let last2 = nexts[nexts.len() - 2..].iter().rfind(|&n| {
                let mut t = n.text();
                if t.is_some() && t.as_deref().unwrap().is_empty() {
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
                Some(v) => v.attr("href").unwrap_or(String::default()),
                None => {
                    let pos = nexts.iter().rposition(|e| {
                        e.attr("href").is_some_and(|h| {
                            cur.trim().ends_with(h.trim())
                                || h.trim() == "#"
                                || format!("{}/1", cur.trim_end_matches('/')).ends_with(h.trim())
                                || format!("{}?page=1", cur.trim_end_matches('/'))
                                    .ends_with(h.trim())
                        })
                    });
                    match pos {
                        Some(p) => {
                            if p < nexts.len() - 1 {
                                nexts[p + 1].attr("href").unwrap()
                            } else {
                                String::default()
                            }
                        }
                        None => String::default(),
                    }
                }
            };
        }
    }
    // if !next.is_empty() && !next[next.rfind('/').unwrap()..].contains(['_', '-', '?']) {
    //     next = String::default();
    // }

    if cur.trim().ends_with(&next_link) || next_link.trim() == "#" || next_link.trim() == "/" {
        next_link = String::default();
    }

    next_link = canonicalize(next_link, scheme, cur);

    tdbg!(next_link)
}

///WebSites `Json` config data
fn website() -> serde_json::Value {
    serde_json::from_str(include_str!("web.json")).unwrap_or_else(|e| {
        quit!("Read `web.json` failed: {}", e);
    })
}

///Save inline/embed `data:image/..+..;..,...` or `base64/url-escaped` content to file.
#[cfg(feature = "embed")]
fn save_to_file(data: &str) {
    if cfg!(not(feature = "embed")) {
        return;
    }
    let t = &format!("{:?}", time::Instant::now());
    let name = &t[t.rfind(':').unwrap() + 2..t.len() - 2];

    use base64::*;
    let ext = &data[..['+', ';', ',']
        .iter()
        .find_map(|&x| data.find(x))
        .unwrap_or(data.len())];
    let offset = &data[data.find(',').unwrap() + 1..];
    let full_name = format!("{name}.{ext}");
    if !path::Path::new(&full_name).exists() {
        {
            if data.contains(";base64,") {
                let mut buf = vec![0; offset.len()];
                let size = engine::general_purpose::STANDARD
                    .decode_slice(offset, &mut buf)
                    .unwrap_or_else(|e| quit!("{e}"));
                buf.truncate(size);
                fs::write(full_name, buf)
            } else {
                fs::write(
                    full_name,
                    percent_encoding::percent_decode_str(offset)
                        .decode_utf8_lossy()
                        .as_ref(),
                )
            }
        }
        .unwrap_or_else(|e| {
            quit!(
                "Write {} to file {name}.{ext} failed: {}",
                &data[..data.find(',').unwrap()],
                e
            )
        });
    }
}

///Show `circle` progress indicator
fn circle_indicator(r: sync::mpsc::Receiver<()>) {
    use io::*;
    use sync::mpsc::*;

    let chars = ['◯', '◔', '◑', '◕', '●'];
    // let chars = ["◯", "◔.", "◑..", "◕...", "●...."];
    let mut o = stdout().lock();

    'l: loop {
        for char in chars {
            print!("{BEG}{char}");
            let _ = o.flush();
            match r.try_recv() {
                Err(TryRecvError::Empty) => (),
                _ => break 'l,
            }
            thread::yield_now();
            thread::sleep(time::Duration::from_millis(200));
        }
    }
    print!("{CL}{BEG}");
    let _ = o.flush();
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
        ["ltr ", "rtl "].map(|x| url = url.trim_start_matches(x));
        url = url.trim_matches(['\'', '"']).trim();
        ["&#39;", "&apos;", "&#34;", "&quot;"]
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
                ".otf", ".ttf", ".woff", ".woff2", ".cur", ".css", ".pdf", ".fnt", ".eot", ".cff",
            ]
            .iter()
            .any(|&ext| url.ends_with(ext))
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
fn css_image(html: &str, scheme: &str, addr: &str) -> collections::HashSet<String> {
    let mut images = collections::HashSet::new();
    CSS.map(|s| {
        let segments = html.split(s);
        if s == "image-set(" {
            for seg in segments.skip(1) {
                images = images
                    .union(&css_image(seg, scheme, addr))
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
                        images.insert(canonicalize(u, scheme, addr));
                    }
                }
            }
        }
    });
    images
}

///Detect terminal emulator using `echo $TERM`
fn terminal_emulator() -> bool {
    env::var("TERM").map_or(false, |o| {
        ["term", "vt", "crt", "pty", "emu", "virt", "onsole"]
            .iter()
            .any(|x| o.contains(x))
    })
}

#[cfg(test)]
mod img {
    use super::*;

    #[test]
    fn detect_terminal_emulator() {
        dbg!(terminal_emulator());
    }

    fn arg(default: &str) -> String {
        let arg = env::args().nth(4);
        arg.unwrap_or(String::from(default))
    }

    #[test]
    fn html() {
        let (html, ..) = get_html(&arg("mmm.red"));
        dbg!(&html);
    }

    #[test]
    fn htmlq() {
        let addr =
            arg("https;://girldreamy.com/xiuren%e7%a7%80%e4%ba%ba%e7%bd%91-no-7689-tang-an-qi/");
        let (html, [img, .., album], _) = get_html(&addr);

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
                .write_all(html.as_bytes())
                .expect("Failed to write stdin.");
            if let Ok(o) = cmd.wait_with_output() {
                if !o.stdout.is_empty() {
                    println!(
                        "Totally found {} <img>",
                        String::from_utf8_lossy(o.stdout.as_ref()).lines().count()
                    );
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

    // fn(..) -> Pin<Box<impl/dyn Future<Output = Something> + '_>>

    #[test]
    fn r#try() {
        // https://bisipic.online/portal.php?page=9 https://xiutaku.com/?start=20

        parse(&arg("https://girldreamy.com"));
    }

    #[test]
    fn css_img() {
        let addr = arg("autodesk.com");
        let (html, _, [scheme, _]) = get_html(&addr);
        let r = css_image(&html, scheme, &addr);
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
        thread::sleep(time::Duration::from_secs(3));
        s.send(()).unwrap_or_else(|e| pl!("send error: {}", e));
    }

    #[test]
    fn run() {
        main();
    }

    #[test]
    fn sanity_check_json() {
        let mut sites = collections::HashSet::new();
        let mut dup_site = vec![];
        let mut img_sel = collections::HashMap::new();

        JSON.get_or_init(website)
            .as_array()
            .expect("Json file parse error.")
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
        dbg!(dup_site);

        let dup_sel = img_sel
            .keys()
            .filter_map(|k| {
                if img_sel[*k].len() > 1 {
                    Some((*k, img_sel[*k].len()))
                } else {
                    None
                }
            })
            // .map(|(k, l)| format!("`{k}` : {l}"))
            .collect::<Vec<_>>();
        pl!(
            "Todally find {} Img Sels, with duplicated {} selectors.",
            img_sel.len(),
            dup_sel.len()
        );
        dbg!(dup_sel);
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
        let data="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNgYAAAAAMAASsJTYQAAAAASUVORK5CYII=";
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
