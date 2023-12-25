use {std::*, util::*};

mod util;

fn main() {
    if env::args().len() > if cfg!(test) { 2 + 3 } else { 2 } {
        quit!("Usage: `Command <URL>`");
    }
    let arg = if cfg!(test) {
        env::args().skip(3).nth(1)
    } else {
        env::args().nth(1)
    }
    .unwrap_or_else(|| {
        quit!("Please input <URL> argument.");
    });

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
    use {serde_json::*, sync::*};

    // static JSON: LazyLock<Value> = LazyLock::new(|| website());
    static JSON: OnceLock<Value> = OnceLock::new();

    let site = JSON
        .get_or_init(website)
        .as_array()
        .expect("Json file parse error.")
        .iter()
        .find(|&s| {
            s["Site"].as_str().map_or(false, |s| {
                s.split_terminator(',')
                    .any(|s| s == host.trim_start_matches("www."))
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
    thread::spawn(|| {
        circle_indicator(r);
    });
    let out = process::Command::new("curl")
        .args([
            addr,
            "--compressed",
            "-e",
            host,
            "-A",
            "Mozilla Firefox",
            "-fsSL",
        ])
        .output()
        .unwrap_or_else(|e| {
            s.send(());
            quit!("curl: {}", e);
        });
    s.send(());
    if out.stdout.is_empty() {
        quit!(
            "Fetch {} failed - {}",
            addr,
            String::from_utf8(out.stderr).unwrap_or_else(|e| e.to_string())
        );
    }
    let res = String::from_utf8_lossy(&out.stdout);
    (res.to_string(), host_info, scheme_host)
}

///Parse photos in web url
fn parse(addr: &str) -> String {
    let (html, [img, mut next_sel, album], [scheme, host]) = get_html(addr);
    let page = crabquery::Document::from(html);
    let imgs = page.select(img.unwrap_or("img[src]"));
    let src = img.map_or("src", |i| {
        if i.trim_end().ends_with(']') {
            &i[i.rfind('[').expect("NO '[' found in <img> selector.") + 1..i.len() - 1]
        } else {
            "src"
        }
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

    (0..2).for_each(|_| {
        t = t[..t.rfind(['/', '-', '_', '|', '–']).unwrap_or(t.len())].trim();
    });

    let slash2dot = t.replace('/', "·");
    t = slash2dot.as_ref();

    let albums = album.map(|a| page.select(a));

    let has_album = album.is_some() && !albums.as_ref().unwrap().is_empty();
    let [albums_len, imgs_len] = [albums.as_ref().map_or(0, |a| a.len()), imgs.len()];

    match (has_album, !imgs.is_empty()) {
        (true, true) => pl!("Totally found <{albums_len}> 📸 and <{imgs_len}> 🏞️  in 📄:{G} {t}"),
        (true, false) => pl!("Totally found <{albums_len}> 📸 in 📄:{G} {t}"),
        (false, true) => pl!("Totally found <{imgs_len}> 🏞️  in 📄:{G} {t}"),
        (false, false) => {
            quit!("∅ 🏞️  found in 📄:{G} {t}");
        }
    }

    t = if t.contains("page") || t.contains('页') {
        t[..t.rfind("page").or_else(|| t.rfind('第')).unwrap_or(t.len())].trim()
    } else {
        t[..t.rfind(['(', ',']).unwrap_or(t.len())].trim()
    };

    let canonicalize_url = |u: String| {
        if !u.starts_with("http") {
            if u.starts_with("//") {
                format!("{scheme}:{u}")
            } else if u.starts_with('/') {
                format!("{scheme}://{host}{u}")
            } else {
                format!("{}/{u}", &addr[..addr.rfind('/').unwrap_or(addr.len())])
            }
        } else {
            u
        }
    };

    match (has_album, !imgs.is_empty()) {
        (_, true) => {
            use collections::*;
            let mut urls = HashSet::new();
            let [mut empty_dup, mut embed] = [0u16; 2];

            for img in imgs {
                let src = img.attr(src).expect("Invalid img[src] selector!");

                if src.starts_with("data:image/") {
                    if cfg!(feature = "embed") {
                        if !urls.insert(src) {
                            empty_dup += 1;
                        }
                    } else {
                        embed += 1;
                    }
                } else {
                    let url = &src[src.rfind("?url=").map(|p| p + 5).unwrap_or(0)..];
                    let clean_url = &url[..url.find('&').unwrap_or(url.len())];

                    // tdbg!(clean_url);
                    if clean_url.trim().is_empty() || !urls.insert(clean_url.to_owned()) {
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
            if !urls.is_empty() {
                download(
                    t,
                    urls.into_iter().map(|url| {
                        if url.starts_with("data:image/") {
                            url
                        } else {
                            canonicalize_url(url)
                        }
                    }),
                    host,
                );
            }
        }
        (true, false) => {
            let mut all = false;

            for (i, alb) in albums.unwrap().iter().enumerate() {
                let mut parse_album = || {
                    let mut href = alb.attr("href").unwrap_or_else(|| {
                        alb.parent()
                            .unwrap()
                            .attr("href")
                            .expect("NO album a[@href] attr found.")
                    });
                    if !href.is_empty() {
                        let album_url = canonicalize_url(href);
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
                    let mut stdin = io::stdin();
                    let mut stdout = io::stdout();

                    let mut t = alb.attr("title").unwrap_or_else(|| {
                        alb.attr("alt").unwrap_or_else(|| {
                            alb.text().map_or_else(
                                || quit!("NO album title can be found."),
                                |x| {
                                    if x.trim().is_empty() {
                                        quit!("Album title is empty.")
                                    } else {
                                        x
                                    }
                                },
                            )
                        })
                    });
                    writeln!(
                        stdout,
                        "{B}Do you want to download Album <{U}{}/{albums_len}{_U}>: {G}{} ?{N}",
                        i + 1,
                        t.trim()
                    );
                    write!(
                        stdout,
                        "{MARK}{B}{Y}Y{u}es⏎{s}N{u}o{s}A{u}ll{s}C{u}ancel: {N}",
                        u = char::from_u32(0x332).unwrap(),
                        s = " | ",
                    );
                    stdout.flush();

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

    next_sel.map_or_else(<_>::default, |n| check_next(page.select(n), addr))
}

///Perform photo download operation
fn download(dir: &str, urls: impl Iterator<Item = String>, host: &str) {
    if cfg!(all(test, not(feature = "download"))) {
        return;
    }

    let path = path::Path::new(dir);
    let create_dir = || {
        if !path.exists() {
            fs::create_dir(path).unwrap_or_else(|e| {
                quit!("Create Dir Error: {}", e);
            });
        }
    };

    let mut curl = process::Command::new("curl");
    curl.current_dir(path).args(["-Z"]);

    #[cfg(feature = "infer")]
    let mut need_file_type_detection = vec![];

    for url in urls {
        if cfg!(feature = "embed") && url.starts_with("data:image/") {
            if let Ok(cur) = env::current_dir() {
                create_dir();
                env::set_current_dir(path);
                save_to_file(url.as_str());
                env::set_current_dir(cur);
            }
            continue;
        }

        let mut name = url[url
            .rfind('/')
            .unwrap_or_else(|| quit!("Invalid Url: {}", url))
            + 1..]
            .trim_start_matches(['-', '_']);
        let has_ext = &name[..name.find('?').unwrap_or(name.len())].rfind('.');

        let mut name_ext = String::default();
        if has_ext.is_none() {
            #[cfg(feature = "infer")]
            {
                need_file_type_detection.push(name.to_owned());
            }
            #[cfg(not(feature = "infer"))]
            {
                name_ext = content_header_info(url.as_ref(), host, name);
            }
        } else {
            name = &name[..name.find('?').unwrap_or(name.len())];
        }

        let file_name = if name_ext.is_empty() {
            name
        } else {
            name_ext.as_str()
        };

        if !path.join(file_name).exists() {
            curl.args([url.as_str(), "-o", file_name]);
        }
    }
    // tdbg!(curl.get_args());
    if curl.get_args().len() == 1 {
        return;
    }

    if cfg!(feature = "curl") {
        create_dir();
        let cmd = curl
            .args([
                "--parallel-immediate",
                "--compressed",
                "-e",
                host,
                "-A",
                "Mozilla Firefox",
                if cfg!(debug_assertions) {
                    "-fsSL"
                } else {
                    "-fsL"
                },
            ])
            .spawn();

        #[cfg(feature = "infer")]
        if !need_file_type_detection.is_empty() {
            cmd.unwrap().wait();
            for f in need_file_type_detection {
                let file = path.join(&f);
                if file.exists() {
                    magic_number_type(file);
                }
            }
        }
    }
}

/// Get `url` content header info to generate full `name.ext`
fn content_header_info(url: &str, host: &str, name: &str) -> String {
    let mut name_ext = String::default();
    tdbg!(url);
    process::Command::new("curl")
        .args([
            url,
            "-e",
            host,
            "-A",
            "Mozilla Firefox",
            "-fsSIL",
            "--compressed",
            "-w",
            "%{content_type}",
        ])
        .output()
        .map_or_else(
            |e| pl!("Get {url} content header info failed: {e}"),
            |o| {
                let header = String::from_utf8_lossy(&o.stdout);
                if let Some(ct) = header.lines().last() {
                    let ext = image_type(ct);
                    name_ext = [name, ext].join(".");
                }
            },
        );
    name_ext
}

/// Infer file type through magic number
#[cfg(feature = "infer")]
fn magic_number_type(pb: path::PathBuf) {
    use io::*;

    let mut f = fs::File::open(&pb).unwrap_or_else(|e| quit!("{e} : {}", pb.display()));
    let mut buf = [0u8; 16];
    f.read_exact(&mut buf);

    let t = infer::get(&buf);
    if let Some(ext) = t {
        let mut new = pb.to_owned();
        new.set_extension(ext.extension());
        fs::rename(pb, new);
    } else {
        let str = String::from_utf8_lossy(&buf);
        if str.contains("<svg") {
            fs::rename(&pb, format!("{}.svg", pb.display()));
        }
    }
}

/// Check `next` selector link page info
fn check_next(nexts: Vec<crabquery::Element>, cur: &str) -> String {
    let mut next_link: String;
    if nexts.is_empty() {
        next_link = String::default();
        //println!("NO next page <element> found.")
    } else if nexts.len() == 1 {
        let element = &nexts[0];
        if element.tag().unwrap() == "span" {
            let items = element.parent().unwrap().children();

            let mut tags = items.split(|e| {
                e.tag().unwrap() == "span"
                // && e.attr("class")
                //     .map_or(true, |c| c.contains("current") || c.contains("now-page"))
            });
            let a = tags
                .next_back()
                .unwrap()
                .iter()
                .filter(|e| e.tag().unwrap() == "a")
                .collect::<Vec<_>>();

            next_link = a
                .first()
                .map_or(String::default(), |f| f.attr("href").unwrap());
        } else {
            next_link = nexts[0].attr("href").unwrap();
        }
    } else {
        let element = &nexts[0];
        if element.tag().unwrap() == "div" && nexts.len() == 2 {
            let tags = element.children();
            let mut rest = tags.split(|tag| {
                tag.children().first().map_or(
                    tag.tag().unwrap() == "span"
                        || tag.attr("class").is_some_and(|c| c.contains("current")),
                    |f| f.attr("class").unwrap().contains("current"),
                )
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

                match t {
                    Some(mut text) => {
                        text.make_ascii_lowercase();
                        text.contains('下') || text.contains("next") || (n.attr("target").is_some())
                    }
                    None => {
                        t = n.attr("title");
                        match t {
                            Some(mut title) => {
                                title.make_ascii_lowercase();
                                title.contains('下') || title.contains("next")
                            }
                            None => {
                                let span = n.select("span.currenttext");
                                if span.is_empty() {
                                    return false;
                                }
                                t = span[0].text();
                                match t {
                                    Some(mut text) => {
                                        text.make_ascii_lowercase();
                                        text.contains('下') || text.contains("next")
                                    }
                                    None => false,
                                }
                            }
                        }
                    }
                }
            });
            next_link = match last2 {
                Some(v) => v
                    .attr("href")
                    .expect("NO [href] attr found in <next> link."),
                None => {
                    let pos = nexts
                        .iter()
                        .rposition(|e| cur.trim().ends_with(&e.attr("href").unwrap().trim()));
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
    if !next_link.is_empty() && !next_link.starts_with("http") {
        if next_link.trim() == "/" || next_link.trim() == "#" {
            next_link = String::default();
        } else {
            next_link = format!(
                "{}{}",
                if next_link.starts_with("//") {
                    &cur[..cur.find("//").unwrap()]
                } else if next_link.starts_with('/') {
                    &cur[..cur[10..].find('/').unwrap() + 10]
                } else {
                    &cur[..=cur.rfind('/').unwrap()]
                },
                next_link
            );
        }
    }

    tdbg!(next_link)
}

///WebSites `Json` config data
fn website() -> serde_json::Value {
    serde_json::from_str(include_str!("web.json")).unwrap_or_else(|e| {
        quit!("Read `web.json` failed: {}", e);
    })
}

///Save inline/embed `data:image/..+..;..,...` or `base64/url-escaped` content to file.
fn save_to_file(data: &str) {
    if cfg!(not(feature = "embed")) {
        return;
    }
    let t = &format!("{:?}", time::Instant::now());
    let name = &t[t.rfind(':').unwrap() + 2..t.len() - 2];
    #[cfg(feature = "embed")]
    {
        use base64::*;
        let ext = image_type(data);
        let offset = &data[data.find(',').unwrap() + 1..];
        let full_name = [name, ext].join(".");
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
                    fs::write(full_name, url_escape::decode(offset).as_bytes())
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
}

///Get content_type info from image url response metadata type
fn image_type(header: &str) -> &str {
    let mut offset = &header[header.find('/').unwrap() + 1..];

    &offset[..offset
        .find('+')
        .or_else(|| offset.find(';'))
        .or_else(|| offset.find(','))
        .unwrap_or(offset.len())]
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
            o.flush();
            match r.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => break 'l,
                Err(TryRecvError::Empty) => (),
            }
            thread::yield_now();
            thread::sleep(time::Duration::from_secs_f32(0.2));
        }
        // print!("{CL}");
        // o.flush();
    }
    print!("{BEG}");
    o.flush();
}

#[cfg(test)]
mod img {
    use super::*;

    #[test]
    fn html() {
        let addr = "mmm.red";
        let (html, ..) = get_html(addr);
        dbg!(&html);
    }

    #[test]
    fn htmlq() {
        let addr = "visualstudio.com";
        let (html, [img, .., album], _) = get_html(addr);
        use process::*;

        let hq = |sel: &str| {
            let cmd = Command::new("htmlq")
                .args([sel])
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

    #[test]
    fn r#try() {
        // https://xiurennvs.xyz https://girldreamy.com/ https://mmm.red
        let arg = env::args().skip(3).nth(1);
        let addr = arg
            .as_deref()
            .unwrap_or("http://www.beautyleg6.com/siwameitui/");

        parse(addr);
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
        s.send(());
    }

    #[test]
    fn run() {
        main();
    }

    #[test]
    fn file_type() {
        let dir = env::current_dir().unwrap();
        let p = dir.join("src/demo");
        #[cfg(feature = "infer")]
        magic_number_type(p);
    }
    #[test]
    fn embed() {
        if cfg!(not(feature = "embed")) {
            return;
        }
        let data="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNgYAAAAAMAASsJTYQAAAAASUVORK5CYII=";

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
