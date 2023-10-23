use {std::*, util::*};

mod util;

fn main() {
    if env::args().len() > 2 {
        exit!("Usage: `Command <URL>`");
    }
    let arg = env::args().nth(1).unwrap_or_else(|| {
        exit!("Please input <URL> argument.");
    });

    let mut next_page = parse(&arg);

    while !next_page.is_empty() {
        next_page = parse(&next_page);
    }
}

///Get `scheme` and `host` info from valid url string
fn check_host(addr: &str) -> [&str; 2] {
    let split = addr.split_once("://").unwrap_or(("http", addr));

    let scheme = split.0;
    if scheme.is_empty()
        || !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
    {
        exit!("Invalid http(s) protocol.");
    }
    let rest = split.1;
    let host = &rest[..rest.find('/').unwrap_or(rest.len())];
    if host.is_empty() || !host.contains('.') {
        exit!("Invalid host info.");
    }
    [scheme, host]
}

///Get `host` info and Generate `img/src/next/album` selector data
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
    println!("{BLINK}{BG}Downloading 📄 ...{N}");
    let out = process::Command::new("curl")
        .args([addr, "-e", host, "-A", "Mozilla Firefox", "-fsSL"])
        .output()
        .unwrap_or_else(|e| {
            exit!("{C}curl: {e}");
        });
    print!("{C}");
    if out.stdout.is_empty() {
        exit!(
            "Fetch `{addr}` failed - {}",
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
    let src = img
        .and_then(|i| {
            if i.trim_end().ends_with(']') {
                Some(
                    &i[i.rfind('[').expect("NO '[' found in img selector.") + 1
                        ..i.rfind(']').unwrap()],
                )
            } else {
                None
            }
        })
        .unwrap_or("src");
    let titles = page.select("title");
    let title = titles
        .first()
        .unwrap_or_else(|| {
            exit!("Not a valid HTML page.");
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

    match (has_album, !imgs.is_empty()) {
        (true, true) => println!(
            "{B}Totally found {} 📸 and {} 🏞️  in 📄:{G} {t}{N}",
            albums.as_ref().unwrap().len(),
            imgs.len(),
        ),
        (true, false) => println!(
            "{B}Totally found {} 📸 in 📄:{G} {t}{N}",
            albums.as_ref().unwrap().len(),
        ),
        (false, true) => println!("{B}Totally found {} 🏞️  in 📄:{G} {t}{N}", imgs.len()),
        (false, false) => {
            exit!("∅ 🏞️  found in 📄:{G} {t}");
        }
    }

    t = if t.contains("page") || t.contains('页') {
        t[..t.rfind("page").or_else(|| t.rfind('第')).unwrap_or(t.len())].trim()
    } else {
        t[..t.rfind(['(', ',']).unwrap_or(t.len())].trim()
    };

    let canonicalize_url = |u: &str| {
        if !u.starts_with("http") {
            if u.starts_with("//") {
                format!("{scheme}:{u}")
            } else if u.starts_with('/') {
                format!("{scheme}://{host}{u}")
            } else {
                format!("{}/{u}", &addr[..addr.rfind('/').unwrap_or(addr.len())])
            }
        } else {
            u.to_owned()
        }
    };
    match (has_album, !imgs.is_empty()) {
        (_, true) => {
            for img in imgs {
                let src = img.attr(src).expect("Invalid img[src] selector!");

                if src.starts_with("data:image/") {
                    #[cfg(feature = "embed")]
                    download(t, &src);
                    continue;
                }

                let mut src = src.as_str();
                src = &src[src.rfind("?url=").map(|p| p + 5).unwrap_or(0)..];
                // src = &src[..src
                //     .rfind("?id=")
                //     .and_then(|id| src.find('&').or(Some(src.len())))
                //     .or(src.rfind('?')).unwrap_or(src.len())];
                src = &src[..src.find('&').unwrap_or(src.len())];
                let file = canonicalize_url(src);
                // tdbg!(&file);
                download(t, &file);
            }
        }
        (true, false) => {
            let mut all = false;

            for (i, alb) in albums.as_ref().unwrap().iter().enumerate() {
                let mut parse_album = || {
                    let mut href = alb.attr("href").unwrap_or_else(|| {
                        alb.parent()
                            .unwrap()
                            .attr("href")
                            .expect("NO a[@href] attr found.")
                    });
                    let album_url = canonicalize_url(&href);
                    let mut next_page = parse(&album_url);
                    while !next_page.is_empty() {
                        next_page = parse(&next_page);
                    }
                };

                if all {
                    parse_album();
                } else {
                    use io::*;
                    let mut stdin = io::stdin();
                    let mut stdout = io::stdout();

                    let mut t = alb.attr("title").unwrap_or_else(|| {
                        alb.attr("alt")
                            .unwrap_or_else(|| alb.text().expect("NO album title can be found."))
                    });
                    writeln!(
                        stdout,
                        "{B}Do you want to download Album <{I}{U}{}/{}{N}>: {B}{G}{} ?{N}",
                        i + 1,
                        albums.as_ref().unwrap().len(),
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
                        exit!("`{e}`");
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
                            println!("{B}Canceled all albums download.{N}");
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
fn download(dir: &str, src: &str) {
    #[cfg(all(feature = "download", any(not(test), feature = "batch")))]
    {
        let path = path::Path::new(dir);
        if (!path.exists()) {
            fs::create_dir(path).unwrap_or_else(|e| {
                exit!("Create Dir Error: `{e}`");
            });
        }

        if src.starts_with("data:image/") {
            #[cfg(feature = "embed")]
            {
                let cur = env::current_dir().unwrap();
                env::set_current_dir(path);
                save_to_file(src);
                env::set_current_dir(cur);
            }
            return;
        }
        let name = src[src.rfind('/').unwrap() + 1..].trim_start_matches(['-', '_']);
        let ext = &name[..name.find('?').unwrap_or(name.len())].find('.');
        let host = &src[..src[10..].find('/').unwrap_or(src.len() - 10) + 10];

        let mut name_ext = String::default();
        if ext.is_none() {
            let header = process::Command::new("curl")
                .args([src, "-e", host, "-A", "Mozilla Firefox", "-fsLI"])
                .output()
                .unwrap_or_else(|e| exit!("Get {src} header info failed: {e}"));

            let info = String::from_utf8_lossy(&header.stdout);
            let ct = "Content-Type: image/";
            let res = info
                .lines()
                .find(|l| l.starts_with(ct))
                .unwrap_or_else(|| exit!("NO `{ct}` of `{src}` found."));

            let image_type = &res[res.find('/').unwrap() + 1
                ..res.find('+').or_else(|| res.find(';')).unwrap_or(res.len())];
            name_ext = [name, image_type].join(".");
        };

        let wget = format!("wget {src} -O {name} --referer={host} -U \"Mozilla Firefox\" -q");
        let curl = format!("curl {src} -o {name} -e {host} -A \"Mozilla Firefox\" -fsL");
        //tdbg!(&curl);
        if (path.exists() && !path.join(name).exists()) {
            #[cfg(feature = "curl")]
            process::Command::new("curl")
                .current_dir(path)
                .args([
                    src,
                    "-o",
                    if name_ext.is_empty() { name } else { &name_ext },
                    "-e",
                    host,
                    "-A",
                    "Mozilla Firefox",
                    "-fsL",
                    //"--location-trusted",
                    #[cfg(feature = "retry")]
                    "--retry 3",
                ])
                .spawn();

            #[cfg(feature = "wget")]
            process::Command::new("wget")
                .current_dir(path)
                .args([
                    src,
                    format!("--referer={host}").as_str(),
                    "-U",
                    "Mozilla Firefox",
                    "-q",
                ])
                .spawn();
        }
    }
}

///Check `next` page info
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
        exit!("`{e}`");
    })
}

///Save inline/embed data:image/..+..;.., base64/url-escaped content to file.
#[cfg(feature = "embed")]
fn save_to_file(data: &str) {
    let ext =
        &data[data.find('/').unwrap() + 1..data.find('+').or_else(|| data.find(';')).unwrap()];

    let t = &format!("{:?}", time::Instant::now());
    let name = &t[t.find(':').unwrap() + 2..t.len() - 2];

    use base64::*;

    if data.contains(";base64,") {
        let mut buf = vec![0; data.len()];
        let size = engine::general_purpose::STANDARD
            .decode_slice(&data[data.find(',').unwrap() + 1..], &mut buf)
            .unwrap_or_else(|e| exit!("{e}"));
        buf.truncate(size);
        fs::write([name, ext].join("."), buf)
            .unwrap_or_else(|e| exit!("Write base64 encoded file {name}.{ext} failed: {e}"));
    } else {
        fs::write(
            [name, ext].join("."),
            url_escape::decode(&data[data.find(',').unwrap() + 1..]).as_bytes(),
        )
        .unwrap_or_else(|e| exit!("Write URL escaped file {name}.{ext} failed: {e}"));
    };
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
        let addr = "bing.com";
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
            cmd.wait_with_output().expect("Failed to get stdout.");
        };

        let i = img.unwrap_or("img[src]");
        println!("{MARK}{B}Image Selector: {HL} {i} {N}",);
        hq(i);

        if let Some(a) = album {
            println!("{MARK}{B}Album Selector: {HL} {a} {N}",);
            hq(a)
        }
    }

    #[test]
    fn r#try() {
        // https://xiurennvs.xyz https://girldreamy.com https://mmm.red

        let addr = "http://www.beautyleg6.com/siwameitui/";
        parse(addr);
    }

    #[test]
    #[cfg(feature = "embed")]
    fn embed_test() {
        let data="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNgYAAAAAMAASsJTYQAAAAASUVORK5CYII=";

        save_to_file(data);
    }

    #[test]
    fn range() {
        let mut addr = "https://girldreamy.com/category/china/xiuren/page/30";
        let page = &addr[addr.rfind('/').unwrap() + 1..];
        let num = page.parse::<u16>().expect("Parse page number failed.");
        (0_u16..=4).map(|i| num - i).for_each(|p| {
            let mut idx = format!("{}{p}", &addr[..=addr.rfind('/').unwrap()]);
            tdbg!(&idx);
            idx = parse(&idx);
            while !idx.is_empty() {
                idx = parse(&idx);
            }
        });
    }
}
