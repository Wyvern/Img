#[cfg(target_os = "hermit")]
use hermit as _;

mod util;
use dom_query::{self as dom, NodeIdProver};
use {std::*, sync::atomic::*, util::*};

static ATTR: &[&str] = &[
    "data-src",
    "data-lazy",
    "data-lazy-src",
    "data-original",
    "data-url",
    "href",
    "src",
    "file",
    "zoomfile",
    "style",
];
static SPINNER: AtomicBool = AtomicBool::new(false);
static SEP: &str = " | ";
static CSS: &[&str] = &["url(", "image(", "image-set("];
static JSON: sync::OnceLock<serde_json::Value> = sync::OnceLock::new();
static CURL: &[&str] = &[
    "--compressed",
    "-kfsL",
    "-A",
    "Mozilla/5.0 Firefox/Edge/Chrome",
    "--tcp-fastopen",
    "--http2",
    #[cfg(debug_assertions)]
    "-S",
    // "-OJ",
];
static IMGS: &[&str] = &[
    ".jpg", ".jpeg", ".jxl", ".png", ".webp", ".bmp", ".tif", ".tiff", ".ico", ".gif", ".svg",
    ".svgz", ".avif", ".heif", ".heic", ".jp2", ".j2k", ".jpx", ".jfif",
];
static TERM: sync::OnceLock<bool> = sync::OnceLock::new();
static mut INALBUM: bool = false;
static mut SUB_DIR: bool = true;
static mut EMBED: bool = false;

fn main() {
    use nanoargs::*;
    let parser = ArgBuilder::new()
        .name("img")
        .version("1.0.0")
        .description("<img> fetcher/cralwer across various web pages.")
        .flag(
            Flag::new("files")
                .desc("Save files directly without create a folder.")
                .short('f'),
        )
        .flag(
            Flag::new("embed")
                .desc("Save embed/inline <svg> and data:image as files.")
                .short('e'),
        )
        .positional(Pos::new("urls").desc("- the url list of web page.").multi())
        .option(
            Opt::new("output")
                .short('o')
                .desc("Output dir where album folder stored in.")
                .validate(Validator::with_hint("must be existed dir", |x| {
                    let p = path::Path::new(x);
                    if p.exists() && p.is_dir() {
                        Ok(())
                    } else {
                        Err("Path must be an existed directory.".into())
                    }
                })),
        )
        .build()
        .unwrap();

    let args = match parser.parse_env() {
        Ok(res) => extract!(res,
            {
            urls:Vec<String> as @pos,
            output:Option<String>,
            files:bool,
            embed:bool
            }
        )
        .unwrap(),
        Err(ParseError::HelpRequested(text) | ParseError::VersionRequested(text)) => {
            print!("{text}");
            quit!("")
        }
        Err(e) => {
            eprintln!("error: {e}");
            quit!("")
        }
    };
    if args.urls.is_empty() {
        print!("{}", parser.help_text());
        return;
    }
    if let Some(dir) = args.output {
        env::set_current_dir(&dir)
            .unwrap_or_else(|x| quit!("Change working directory to {} failed: {} !", &dir, x))
    }

    if args.files {
        unsafe {
            SUB_DIR = false;
        }
    }
    if args.embed {
        unsafe {
            EMBED = true;
        }
    }
    let urls = args.urls.into_iter().collect::<collections::HashSet<_>>();
    for u in urls {
        let mut _next_page = parse(&u);
        #[cfg(not(test))]
        {
            while !_next_page.is_empty() {
                _next_page = parse(&_next_page);
            }
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
fn host_info(host: &str) -> [Option<&str>; 5] {
    let site = JSON.get_or_init(website)["Sites"]
        .as_array()
        .unwrap()
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
    site.map_or([None; 5], |s| {
        ["Img", "Next", "Album", "Title", "Page"].map(|key| s[key].as_str().map(|v| v.trim()))
    })
}

///Fetch web page generate html content
fn get_html(addr: &str) -> (String, usize) {
    _ = io::stdout().lock();
    let h = thread::spawn(|| {
        SPINNER.store(true, Ordering::Release);
        circle_indicator();
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
            SPINNER.store(false, Ordering::Release);
            quit!("curl: {}", e);
        });
    SPINNER.store(false, Ordering::Release);
    if !out.stderr.is_empty() {
        quit!(
            "Fetch {} failed : {}",
            addr,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    let ll = s.rfind('\n').unwrap();
    h.join().unwrap();
    (s, ll)
}

///Parse photos in web url
fn parse(addr: &str) -> String {
    let (html, ll) = get_html(addr);
    let url_effective = &html[ll + 1..];
    let host = check_host(url_effective);
    let [
        mut img,
        mut next_sel,
        mut album,
        mut title_sel,
        mut page_sel,
    ] = host_info(host);
    let page = dom::Document::from(&html[..ll]);

    if img.is_none() {
        let cat = JSON.get_or_init(website)["Series"].as_array().unwrap();
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
            [img, next_sel, album, title_sel, page_sel] = host_info(series[2]);
        }
    }

    let sels = img.and_then(|i| i.split_once(SEP));
    let sel = sels.map(|(l, _)| l.trim()).or(img);

    let mut json_img = collections::HashSet::new();
    let mut html_img = dom::Selection::default();
    let css_img = if img.is_none() {
        css_image(html.as_ref(), addr)
    } else {
        collections::HashSet::new()
    };
    let doc;
    let mut jsontitle = String::default();
    let query = page_sel.is_some_and(|p| p.starts_with("https://"));
    if sel.is_some_and(|s| s.starts_with("json:")) {
        let kind = sel.unwrap().trim_start_matches("json:").trim();
        let name = sels.map(|(_, r)| r).unwrap().trim();
        let script = page.select("script");
        for s in script.iter().filter(|s| !s.immediate_text().is_empty()) {
            let t = s.immediate_text();
            if query {
                let api = page_sel.unwrap();
                let mut query = api.to_owned();
                api.split('{')
                    .skip(1)
                    .filter_map(|part| part.split('}').next())
                    .for_each(|k| {
                        t.split_once(k)
                            .and_then(|(_, r)| r.split_once(k.chars().last().unwrap()))
                            .into_iter()
                            .for_each(|(v, _)| query = query.replace(&format!("{{{k}}}"), v));
                    });
                if !query.contains('{') {
                    tdbg!(&query);
                    let (data, pos) = get_html(&query);
                    let json = &data[..pos];
                    let jv: &serde_json::Value = &serde_json::from_str(json).unwrap();
                    if let Some(t) = title_sel {
                        let sel = t.split_once(SEP).unwrap().1.trim();
                        jsontitle = jv.pointer(sel).unwrap().as_str().unwrap().into();
                    }

                    let pat = name.split_once("->");
                    let data = jv
                        .pointer(pat.map(|(p, _)| p).unwrap_or(name).trim())
                        .unwrap()
                        .as_str()
                        .unwrap();
                    doc = dom::Document::from(data);
                    html_img = doc.select("img[src]");
                    if html_img.is_empty() {
                        data.split("[img]")
                            .skip(1)
                            .filter_map(|x| x.split("[/img]").next())
                            .for_each(|v| {
                                if let Some((_, sub)) = pat {
                                    json_img.insert(sub.trim().replace("{?}", v));
                                } else {
                                    json_img.insert(v.into());
                                }
                            });
                    }
                    break;
                }
            } else {
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
                        "var" => {
                            u.split('\'')
                                .nth(1)
                                .unwrap()
                                .split("https://")
                                .skip(1)
                                .for_each(|url| {
                                    json_img.insert(format!("https://{url}"));
                                });
                            break;
                        }
                        _ => (),
                    }
                }
            }
        }
    } else {
        html_img = page.select(sel.unwrap_or("img"));
    }

    let mut source_img = dom::Selection::default();
    if img.is_none() {
        html_img = html_img.add("image").add("input[type='image']");
        if unsafe { EMBED } {
            html_img = html_img.add("svg");
        }
        source_img = page.select("source[srcset]");
    }

    let titles = page.select(if !json_img.is_empty() {
        "script"
    } else {
        "title"
    });

    let mut albums = album.map(|a| page.select(a));
    let has_album = albums.as_ref().is_some_and(|a| !a.is_empty());
    let page_title = || titles.first().immediate_text();
    let title = title_sel.map_or_else(
        || {
            if !json_img.is_empty() {
                titles
                    .iter()
                    .find_map(|s| {
                        ["metaKeywords", "title="].iter().find_map(|kw| {
                            s.immediate_text().split_once(kw).map(|(_, v)| v.to_owned())
                        })
                    })
                    .unwrap()
                    .split('"')
                    .nth(1)
                    .unwrap()
                    .split(',')
                    .max_by_key(|&seg| seg.trim().len())
                    .unwrap()
                    .into()
            } else {
                page_title()
            }
        },
        |t| {
            if has_album {
                page_title()
            } else {
                if jsontitle.is_empty() {
                    if query {
                        page_title()
                    } else {
                        page.select_single(t).immediate_text()
                    }
                } else {
                    jsontitle.into()
                }
            }
        },
    );

    let mut t = if title_sel.is_some() {
        &title
    } else {
        title
            .rsplitn(5, ['/', '-', '_', '|', '–'])
            .skip(1)
            .max_by_key(|x| x.trim().len())
            .unwrap_or(&title)
    }
    .trim();

    let [albums_len, imgs_len, json_len] = [
        albums.as_ref().map_or(0, |a| a.length()),
        html_img.length() + source_img.length() + css_img.len() + json_img.len(),
        json_img.len(),
    ];

    if has_album && imgs_len == 0 {
        unsafe {
            INALBUM = true;
        }
    }
    let term_title = if *TERM.get_or_init(|| {
        std::env::var("TERM").is_ok_and(|o| {
            ["term", "vt", "crt", "pty", "emu", "virt", "onsole"]
                .iter()
                .any(|x| o.contains(x))
        })
    }) {
        format_args!("{G} \x1b]8;;{addr}\x1b\\{t}\x1b]8;;\x1b\\")
    } else {
        format_args!("{G} {t}")
    };

    let name_count = |name: &[&str], count: &[usize]| -> String {
        name.iter()
            .zip(count)
            .filter_map(|(&n, &c)| {
                if c > 0 {
                    let cnt = if c == count.iter().sum::<usize>() {
                        format_args!("")
                    } else {
                        format_args!("({c})")
                    };
                    Some(format!("{n}{cnt}"))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(" + ")
    };

    let htj = name_count(
        ["HTML", "CSS", "JSON"].as_slice(),
        [
            html_img.length() + source_img.length(),
            css_img.len(),
            json_len,
        ]
        .as_slice(),
    );
    let prefix = "✔︎ Totally found";
    match (has_album, imgs_len > 0) {
        (true, true) => {
            pl!("{prefix} <{albums_len}> 📸 and <{imgs_len}: {htj}> 🏞️  in 📄:{term_title}")
        }

        (true, false) => pl!("{prefix} <{albums_len}> 📸 in 📄:{term_title}"),
        (false, true) => pl!("{prefix} <{imgs_len}: {htj}> 🏞️  in 📄:{term_title}"),
        (false, false) => quit!("∅ 🏞️  found in 📄:{term_title}"),
    }

    t = if let Some(p) = t
        .as_bytes()
        .windows(" page".len())
        .rposition(|w| w.eq_ignore_ascii_case(" page".as_bytes()))
    {
        &t[..p]
    } else {
        if t.contains('页') {
            &t[..t.rfind('第').unwrap_or(t.len())]
        } else {
            &t[..t.rfind(['(', ',']).unwrap_or(t.len())]
        }
        .trim()
    };

    match (has_album, imgs_len > 0) {
        (_, true) => {
            let mut urls = collections::HashSet::<String>::new();
            let [mut empty, mut dup, mut _embed] = [0usize; 3];
            let mut handle_embed = |s: &str| {
                if s.starts_with("data:image/") || s.starts_with("<svg ") {
                    if unsafe { EMBED } {
                        if !urls.insert(s.into()) {
                            dup += 1;
                        }
                    } else {
                        _embed += 1;
                    }
                } else if s.is_empty() {
                    empty += 1;
                } else if !urls.insert(normarlize(s, addr)) {
                    dup += 1;
                }
            };

            for elm in html_img.nodes() {
                if elm.node_name().is_some_and(|n| n.as_ref() == "svg") {
                    if !elm.has_attr("xmlns") && !elm.has_attr("xmlns:xlink") {
                        elm.set_attr("xmlns", "http://www.w3.org/2000/svg");
                    }
                    let text = elm.html();
                    handle_embed(text.as_ref());
                    continue;
                }
                let value = ATTR.iter().find_map(|a| elm.attr(a));

                match value {
                    Some(val) => {
                        if sel.is_some_and(|x| x.ends_with("[style]")) {
                            let x = CSS
                                .iter()
                                .find_map(|&s| val.trim().split_once(s))
                                .and_then(|frag| url_image(frag.1))
                                .unwrap_or_default();
                            handle_embed(x.as_ref());
                        } else if sel == img {
                            handle_embed(url_redirect_and_query_cleanup(&val).as_ref());
                        } else {
                            handle_embed(&val);
                        }
                    }
                    None => handle_embed(""),
                }
            }

            for e in source_img.nodes() {
                let multi_urls = e.attr("srcset").unwrap();
                let mut url = multi_urls
                    .split(",")
                    .max_by_key(|x| x.len())
                    .unwrap()
                    .trim();
                if let Some((l, _)) = url.rsplit_once(' ') {
                    url = l;
                }
                handle_embed(url_redirect_and_query_cleanup(url).as_ref());
            }

            let skip = empty + dup + _embed;
            if skip > 0 {
                let edm = name_count(
                    ["Empty", "Duplicated", "Embed"].as_slice(),
                    [empty, dup, _embed].as_slice(),
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
                        let mut args: Vec<&str> = Vec::new();
                        args.extend(urls.iter().map(|s| s.as_str()));
                        args.extend(CURL);
                        args.extend(["-Z", "--parallel-immediate"]);
                        let o = run_cmd("curl", &args, &[]);
                        let html = String::from_utf8_lossy(&o);
                        let page = dom::Document::from(html.as_ref());
                        let html_img = page.select(r);
                        urls.clear();
                        for e in html_img.nodes() {
                            let src = e.attr("src").unwrap();
                            let title_alt = ["title", "alt"].iter().find_map(|a| {
                                e.attr(a).and_then(|x| {
                                    let attr = x.trim();
                                    if !attr.is_empty()
                                        && IMGS.iter().any(|&ext| {
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
                            let cano = || normarlize(&src, addr);
                            urls.insert(
                                title_alt.map_or_else(cano, |x| format!("{}{SEP}{x}", cano())),
                            );
                        }
                    }
                    _ => (),
                }
            }
            // tdbg!(&urls, &css_img, &json_img;);
            download(t, urls.into_iter().chain(css_img).chain(json_img), host)
        }
        (true, false) => {
            let mut all = false;

            for (i, alb) in albums.take().unwrap().iter().enumerate() {
                let parse_album = || {
                    let href = alb.attr("href").unwrap_or_else(|| {
                        let mut p = alb.parent();
                        let mut href = None;
                        let mut n = 2;
                        while n > 0 {
                            href = p.attr("href");
                            if href.is_some() {
                                break;
                            }
                            n -= 1;
                            if n > 0 {
                                p = p.parent();
                            }
                        }

                        href.unwrap_or_else(|| p.select("a[href]").first().attr("href").unwrap())
                    });

                    if !href.is_empty() {
                        let album_url = normarlize(&href, addr);
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
                            let t = alb.immediate_text();
                            if t.trim().is_empty() {
                                quit!("Album title text is empty.")
                            } else {
                                t.lines()
                                    .max_by_key(|l| l.trim().len())
                                    .unwrap()
                                    .trim()
                                    .into()
                            }
                        });

                    _ = writeln!(
                        stdout,
                        "{B}Do you want to download Album <{U}{}/{albums_len}{_U}>: {G}{}{N}{B}?{N}",
                        i + 1,
                        t.trim(),
                    );
                    _ = write!(
                        stdout,
                        "{MARK}{B}{Y}Y{u}es⏎{s}N{u}o{s}A{u}ll{s}C{u}ancel ␛ : {N}",
                        u = char::from_u32(0x332).unwrap(),
                        s = SEP,
                    );
                    _ = stdout.flush();

                    #[cfg(unix)]
                    {
                        use termion::event::Key;
                        use termion::input::TermRead;
                        use termion::raw::IntoRawMode;

                        let mut o = stdout.into_raw_mode().unwrap();
                        let clear = || {
                            write!(o, "{CL}").unwrap();
                            o.flush().unwrap();
                            drop(o);
                        };
                        match stdin.keys().next() {
                            Some(Ok(Key::Char('y') | Key::Char('Y') | Key::Char('\n'))) => {
                                clear();
                                parse_album()
                            }
                            Some(Ok(Key::Char('n') | Key::Char('N'))) => {
                                clear();
                                next_sel = None;
                                continue;
                            }
                            Some(Ok(Key::Char('a') | Key::Char('A'))) => {
                                clear();
                                all = true;
                                parse_album()
                            }
                            _ => {
                                clear();
                                pl!("⤴ Canceled all albums download.");
                                next_sel = None;
                                page_sel = None;
                                break;
                            }
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        let mut input = String::new();
                        stdin.read_line(&mut input).unwrap_or_else(|e| {
                            quit!("{}", e);
                        });
                        input.make_ascii_lowercase();
                        let mut clear = || {
                            write!(stdout, "{UP}{CL}").unwrap();
                            stdout.flush().unwrap();
                        };
                        match input.trim() {
                            "y" | "yes" | "" => {
                                clear();
                                parse_album()
                            }
                            "n" | "no" => {
                                clear();
                                next_sel = None;
                                continue;
                            }
                            "a" | "all" => {
                                clear();
                                all = true;
                                parse_album()
                            }
                            _ => {
                                clear();
                                pl!("⤴ Canceled all albums download.");
                                next_sel = None;
                                page_sel = None;
                                break;
                            }
                        };
                    }
                }
            }
        }
        _ => (),
    }

    if has_album && imgs_len == 0 {
        unsafe {
            INALBUM = false;
        }
    }

    next_sel.map_or_else(
        || {
            page_sel.map_or_else(<_>::default, |p| {
                if !query && albums.is_none_or(|a| a.is_empty()) && unsafe { !INALBUM } {
                    pause();
                    check_next(p, addr, &page)
                } else {
                    String::default()
                }
            })
        },
        |n| {
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
                check_next(n, addr, &page)
            }
        },
    )
}

///Normalize `img/next` link `url` in `addr`
fn normarlize(url: &str, addr: &str) -> String {
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
    cfg_select! {
        target_os = "macos"=> {name.replace(":", "|")}
        any(all(unix, not(target_os = "macos")), target_family = "wasm")=>{name.replace("/", "_")}
        target_family = "windows"=>{name.chars()
            .map(|c| match c {
                '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                _ => c,
            })
            .collect()}
        _=>{name.into()}
    }
}
///Perform photo download operation
fn download(dir: &str, urls: impl Iterator<Item = String>, host: &str) {
    if cfg!(all(test, not(feature = "download"))) {
        return;
    }

    let os_path = sanitize_path(dir);
    let path = path::Path::new(&os_path);
    let create_dir = || {
        if unsafe { SUB_DIR } && !path.exists() {
            fs::create_dir(path).unwrap_or_else(|e| {
                quit!("Create Dir Error: {}", e);
            });
        }
    };

    let mut curl = process::Command::new("curl");
    if unsafe { SUB_DIR } {
        curl.current_dir(path);
    }

    let mut no_ext = collections::HashMap::new();
    #[cfg(not(unix))]
    let mut no_ext_curl = process::Command::new("curl");
    #[cfg(not(unix))]
    no_ext_curl.args([
        "-Z",
        "--parallel-immediate",
        "-sILo",
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
        if unsafe { EMBED } && (url.starts_with("data:image/") || url.starts_with("<svg ")) {
            {
                if let Ok(cur) = env::current_dir() {
                    if unsafe { SUB_DIR } {
                        create_dir();
                        env::set_current_dir(path).unwrap();
                    }

                    save_to_file(url.as_str());

                    if unsafe { SUB_DIR } {
                        env::set_current_dir(cur).unwrap();
                    }
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
        let mut name_ext = String::default();
        if has_ext.is_none() {
            lr.map_or_else(
                || {
                    #[cfg(not(unix))]
                    no_ext_curl.arg(&url);
                    no_ext.insert(url.clone(), name.to_owned());
                },
                |(_, file_name)| name_ext = file_name.into(),
            )
        } else {
            name = name_no_query
        }
        if no_ext.contains_key(&url) {
            continue;
        }

        let file_name = if name_ext.is_empty() {
            name.trim_end_matches("!lrg")
        } else {
            name_ext.as_str()
        };

        let enc_url = percent_encoding::utf8_percent_encode(u, nan).to_string();

        // tdbg!(&url, &enc_url;);
        curl.args([&enc_url, "-o", file_name]);
    }

    // tdbg!(&no_ext);
    let opts = [
        "-e",
        &format!("https://{host}"),
        "--retry",
        "3",
        "-Z",
        "--parallel-immediate",
        "-C-",
    ];
    // tdbg!(curl.get_args().len() / 3, no_ext.len(););
    if curl.get_args().len() > 0 {
        create_dir();
        _ = curl.args(CURL).args(opts).spawn();
    }

    if !no_ext.is_empty() {
        create_dir();
        curl = process::Command::new("curl");
        if unsafe { SUB_DIR } {
            curl.current_dir(path);
        }

        #[cfg(unix)]
        {
            use fork::*;
            match fork() {
                Ok(Fork::Child) => {
                    curl.args(no_ext.iter().flat_map(|(u, f)| [u, "-o", f]));
                    curl.args(CURL).args(opts).status().unwrap();
                    for (_, f) in no_ext {
                        let file = path.join(&f);
                        if file.is_file() {
                            magic_number_type(file);
                        }
                    }
                    process::exit(0);
                }
                Err(e) => quit!("Fork process failed: {e}"),
                _ => (),
            }
        }
        #[cfg(not(unix))]
        {
            no_ext_curl.output().map_or_else(
                |e| pl!("Query content-type info failed: {}", e),
                |o| {
                    let res = String::from_utf8_lossy(&o.stdout);
                    for (mut url, mut content_type) in
                        res.lines().filter_map(|l| l.split_once("|->"))
                    {
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
}

/// Infer file type through magic number
#[cfg(unix)]
fn magic_number_type(pb: path::PathBuf) {
    use file_format::*;
    let t = FileFormat::from_file(&pb);
    fs::rename(
        &pb,
        pb.with_extension(t.map_or_else(|_| "ext!".to_owned(), |ty| ty.extension().into())),
    )
    .unwrap_or_else(|e| pl!("Rename {} failed: {}", pb.display(), e));
}

/// Check `next` selector link page info
fn check_next(next: &str, cur: &str, page: &dom::Document) -> String {
    let mut next_link = dom::Document::default().text();
    let ns = next.split_once(SEP);
    let nxt = ns.map_or(next, |(l, _)| l);
    let attr = "href";
    let set_next = |tags: &[dom::NodeRef]| {
        let tag = tags.iter().find(|e| {
            e.node_name().is_some_and(|n| n.as_ref() == "a")
                || e.children()
                    .iter()
                    .find_map(|c| c.node_name())
                    .is_some_and(|n| n.as_ref() == "a")
        });

        tag.filter(|e| !e.is_empty_element())
            .and_then(|e| {
                e.attr("href")
                    .or_else(|| e.children().iter().find_map(|c| c.attr("href")))
            })
            .unwrap_or_default()
    };

    let mut nexts = page.select(nxt).nodes().to_vec();
    nexts.sort_by_key(|x| x.attr(attr));
    nexts.dedup_by_key(|x| x.attr(attr));
    tdbg!(nexts.len());
    if nexts.is_empty() {
        tdbg!(nxt);
    } else if nexts.len() == 1 {
        let element = nexts[0];
        if element.node_name().is_some_and(|n| n.as_ref() == "span") || element.attr(attr).is_none()
        {
            let items = element.parent().unwrap().children();
            let tags = items
                .split(|e| (&element).node_id() == e.node_id())
                .next_back()
                .unwrap();
            if !tags.is_empty() {
                next_link = set_next(tags);
            }
        } else if element.node_name().is_some_and(|n| n.as_ref() == "i") {
            next_link = element.parent().unwrap().attr(attr).unwrap();
        } else {
            next_link = element.attr(attr).unwrap();
        }
    } else {
        let last2 = nexts.iter().rev().take(2).find(|n| {
            let txt = n.immediate_text();
            let t = txt.as_ref();
            let next_下 = |t: &str| {
                t.contains('下') || t.split_whitespace().any(|w| w.eq_ignore_ascii_case("next"))
            };

            if !t.is_empty() {
                next_下(t) || n.attr("target").is_some()
            } else {
                match n.attr("title") {
                    Some(title) => next_下(title.as_ref()),
                    None => {
                        let s = dom::Selection::from(**n);
                        let span = s.select("span.currenttext");
                        if span.is_empty() {
                            return false;
                        }
                        span.iter()
                            .find(|s| !s.immediate_text().is_empty())
                            .is_some_and(|s| next_下(s.immediate_text().as_ref()))
                    }
                }
            }
        });
        match last2 {
            Some(v) => next_link = v.attr(attr).unwrap_or_default(),
            None => {
                let pos = nexts.iter().rposition(|e| {
                    e.attr(attr).is_some_and(|h| {
                        let href = h.trim();
                        cur.trim().ends_with(href)
                            || h.trim() == "#"
                            || ["/1", "?page=1", "/page/1"].iter().any(|suffix| {
                                format!("{}{suffix}", cur.trim_end_matches('/')).ends_with(href)
                            })
                            || e.immediate_text().contains("<")
                    })
                });
                if let Some(p) = pos
                    && p < nexts.len() - 1
                {
                    next_link = nexts.get(p + 1).unwrap().attr(attr).unwrap()
                }
            }
        };
    }
    // if !next.is_empty() && !next[next.rfind('/').unwrap()..].contains(['_', '-', '?']) {
    //     next = String::default();
    // }

    if !next_link.is_empty() && cur.trim().ends_with(next_link.as_ref())
        || next_link.trim() == "#"
        || next_link.trim() == "javascript:;"
        || next_link.trim() == "/"
    {
        next_link = dom::Document::default().text();
    }
    let mut ret = String::default();
    if !next_link.is_empty() {
        ret = ns.map_or_else(
            || normarlize(next_link.as_ref(), cur),
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

    tdbg!(ret)
}

///Run arbitrary command in sync mode
fn run_cmd(cmd: &str, args: &[&str], data: &[u8]) -> Vec<u8> {
    let mut child = process::Command::new(cmd)
        .args(args)
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .spawn()
        .unwrap();

    if !data.is_empty() {
        use std::io::Write;
        child.stdin.as_mut().unwrap().write_all(data).unwrap();
    }

    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    out.stdout
}

///WebSites `Json` config data
fn website() -> serde_json::Value {
    let data = cfg_select! {
        unix=>{
            run_cmd("gzip", &["-dc"], include_bytes!("../web.cbor.gz"))
        }
        windows=>{
            run_cmd("tar", &["-xOzf", "-"], include_bytes!("../web.tar.gz"))
        }
        _=>{
            *include_bytes!("../web.cbor")
        }
    };
    cbor4ii::serde::from_slice(&data).unwrap_or_else(|e| {
        quit!("Read `web.cbor` failed: {}", e);
    })
}

///Save inline/embed `data:image/..+..;base64,...` or `base64/url-escaped` or <svg> content to file.
fn save_to_file(data: &str) {
    let generate_name = |ext: &str| -> String { format!("{}.{ext}", fastrand::u32(..)) };

    if data.starts_with("<svg ") {
        let mut full_name = generate_name("svg");
        //Prevent overwriting other images with the same file name.
        while path::Path::new(&full_name).exists() {
            full_name = generate_name("svg");
        }
        fs::write(&full_name, data)
            .unwrap_or_else(|e| quit!("Write <svg> to file {full_name} failed: {}", e));
    } else {
        let ctx = &data["data:image/".len()..data.find(',').unwrap()];
        let ext = &ctx[..['+', ';']
            .iter()
            .find_map(|&x| ctx.find(x))
            .unwrap_or(ctx.len())];

        let mut full_name = generate_name(ext);
        //Prevent overwriting other images with the same file name.
        while path::Path::new(&full_name).exists() {
            full_name = generate_name(ext);
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
}

///Show `circle` progress indicator
fn circle_indicator() {
    use io::*;
    let chars = ['◯', '◔', '◑', '◕', '●'];
    let mut o = stdout().lock();
    let t = time::Instant::now();
    'l: while SPINNER.load(Ordering::Acquire) {
        let secs = t.elapsed().as_secs();
        let time = if secs > 0 {
            format_args!("{secs:>2}s")
        } else {
            format_args!("")
        };
        for char in chars {
            print!("{CL}{char}..{time}");
            o.flush().unwrap();
            if !SPINNER.load(Ordering::Acquire) {
                break 'l;
            }
            thread::sleep(time::Duration::from_millis(200));
        }
    }
    print!("{CL}");
    o.flush().unwrap();
}

///cleanup url
fn url_redirect_and_query_cleanup<'a>(url: &'a str) -> borrow::Cow<'a, str> {
    use {borrow::Cow, percent_encoding::*};
    let dec_url = percent_decode_str(url).decode_utf8_lossy();
    fn cleanup(s: &str) -> &str {
        let c = &s[s.rfind("?url=").map_or(0, |p| p + 5)..];
        &c[..c
            .find('?')
            .and_then(|q| c[q..].find('&').map(|a| a + q))
            .or_else(|| {
                c.rfind('/').and_then(|slash| {
                    c[slash..].rfind('.').and_then(|dot| {
                        c[slash + dot..]
                            .find(['&', '='])
                            .map(|amp| amp + dot + slash)
                    })
                })
            })
            .unwrap_or(c.len())]
    }
    match dec_url {
        Cow::Borrowed(s) => Cow::Borrowed(cleanup(s)),
        Cow::Owned(s) => Cow::Owned(cleanup(&s).into()),
    }
}

///Parse inline `url(),image()`
fn url_image<'a>(content: &'a str) -> Option<borrow::Cow<'a, str>> {
    let mut url = content;
    for x in ["ltr ", "rtl "] {
        url = url.trim_start_matches(x);
    }
    url = url.trim_matches(['\'', '"']).trim();
    for x in ["&#39;", "&apos;", "&#34;", "&quot;"] {
        url = url.trim_start_matches(x).trim_end_matches(x).trim();
    }

    use borrow::Cow;
    if url.starts_with("data:image/") {
        return Some(Cow::Borrowed(url));
    }

    let dec = url_redirect_and_query_cleanup(url);
    fn validate_url(mut url: &str) -> Option<&str> {
        url = url[..url.rfind("#xywh").unwrap_or(url.len())].trim();
        if url.is_empty()
            || url == "undefined"
            || url.starts_with(['{', '$'])
            || url.contains('#')
            || IMGS.iter().all(|&ext| {
                !url.rfind('.')
                    .is_some_and(|dot| url[dot..].eq_ignore_ascii_case(ext))
            })
        {
            None
        } else {
            Some(url)
        }
    }
    match dec {
        Cow::Borrowed(s) => validate_url(s).map(Cow::Borrowed),
        Cow::Owned(s) => validate_url(&s).map(|v| Cow::Owned(v.into())),
    }
}

///Get `page` css style `url(),image(),image-set()`
fn css_image(html: &str, addr: &str) -> collections::HashSet<String> {
    let mut images = collections::HashSet::new();
    let mut add_image = |seg: &str| {
        if let Some(u) = url_image(seg) {
            if u.starts_with("data:image/") {
                if unsafe { EMBED } {
                    images.insert(u.into());
                }
            } else {
                images.insert(normarlize(&u, addr));
            }
        }
    };
    fn balanced_extract(s: &str) -> &str {
        let mut bp = 1;
        let mut inq = false;
        for (i, c) in s.char_indices() {
            match c {
                '\'' | '"' if !s[..i].ends_with('\\') => {
                    inq = !inq;
                }
                '(' if !inq => bp += 1,
                ')' => {
                    if !inq {
                        bp -= 1;
                    }
                    if bp == 0 {
                        return s[..i].trim();
                    }
                }
                _ => (),
            }
        }
        s
    }
    for &style in CSS {
        let segments = html.split(style);
        if style == "image(" || style == "image-set(" {
            for mut seg in segments.skip(1) {
                seg = balanced_extract(seg);
                for s in seg.split(",") {
                    if let Some(url) = s
                        .trim()
                        .split_ascii_whitespace()
                        .find(|u| !u.trim_matches(['\'', '"']).trim().is_empty())
                        && !url.starts_with("url(")
                    {
                        add_image(url);
                    }
                }
            }
        } else {
            for mut seg in segments.skip(1) {
                seg = balanced_extract(seg);
                add_image(seg);
            }
        }
    }
    images
}

#[cfg(test)]
mod img {
    use super::*;

    #[test]
    fn html() {
        let arg = arg(current_fn!());
        let addr = arg.as_deref().unwrap_or_else(|| "mmm.red");
        let html = get_html(addr);
        dbg!(&html);
    }

    #[test]
    fn htmlq() {
        let arg = arg(current_fn!());
        let addr = arg.as_deref().unwrap_or_else(|| "https://bisipic.online/");
        let host = check_host(addr);
        let (html, ll) = get_html(addr);
        let [img, _, album, ..] = host_info(host);
        use io::*;

        let i = img.unwrap_or("img[src]");
        pl!("{MARK} Image Selector: {HL} {i} ");
        let mut r = run_cmd("htmlq", &[i], html[..ll].as_ref());
        println!("Totally found {} <img>", r.lines().count());

        if let Some(a) = album {
            pl!("{MARK} Album Selector: {HL} {a} ");
            r = run_cmd("htmlq", &[a], html[..ll].as_ref());
            println!("{}", String::from_utf8_lossy(&r));
        }
    }

    #[test]
    fn mut_val() {
        let var = 123;
        mv!(var = 100 * 2 + 22);
        tdbg!(var);
    }

    #[test]
    fn cbor() {
        use fs::*;
        use io::*;

        let json_file = File::open("src/web.json").unwrap();
        let reader = BufReader::new(json_file);
        let value: serde_json::Value = serde_json::from_reader(reader).unwrap();

        let cbor_file = File::create("web.cbor").unwrap();
        let writer = BufWriter::new(cbor_file);
        cbor4ii::serde::to_writer(writer, &value).unwrap();

        run_cmd("gzip", &["-kf", "web.cbor"], &[]);
    }

    // fn(..) -> Pin<Box<impl/dyn Future<Output = Something> + '_>>

    fn arg(f: &str) -> Option<String> {
        env::args()
            .skip(1)
            .find(|a| !a.starts_with("--") && !f.ends_with(a))
    }

    #[test]
    fn run() {
        if let Some(arg) = arg(current_fn!()) {
            parse(&arg);
        } else {
            [
                "https://xiuren.biz/latest-post/",
                "https://bisipic.online",
                "https://bestgirlsexy.com/category/china/imiss/page/7/",
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
    fn css() {
        let s = r#"dumy text image('b'".jxl' 3x ,url("a(b.png") type("image/png"), url( 'c\"x.png') 1x, '   c)'d"(.png     ' 2x , url( x.png )  ,  url(' o\'k.jpg ") )"#;
        css_image(s, "demo.com").dbg();
    }

    #[test]
    fn progress() {
        thread::spawn(|| {
            SPINNER.store(true, Ordering::Release);
            circle_indicator();
        });
        thread::sleep(time::Duration::from_secs(5));
        SPINNER.store(false, Ordering::Release);
    }

    #[test]
    fn sanity_check_dup() {
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
        let dir = path::Path::new("Search");
        for f in fs::read_dir(dir).unwrap() {
            let p = f.unwrap().path();
            magic_number_type(p);
        }
    }

    #[test]
    fn embed() {
        let data = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNgYAAAAAMAASsJTYQAAAAASUVORK5CYII=";
        save_to_file(data);
    }
}
