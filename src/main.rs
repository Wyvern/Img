use std::*;

mod util;
use util::*;

fn main() {
    let arg = env::args().nth(1).unwrap_or_else(|| {
        exit(format_args!("Please input URL argument.."));
    });

    let mut next = parse(&arg);

    while !next.is_empty() {
        //dbg!(&next);
        next = parse(&next);
    }
}

///Get `scheme` and `host` info from valid url string
fn check_host(addr: &str) -> [&str; 2] {
    let split = addr.split_once("://").unwrap_or(("http", addr));

    let scheme = split.0;
    if scheme.is_empty()
        || !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
    {
        exit(format_args!("{R}Invalid http(s) protocol.{N}"));
    }
    let rest = split.1;
    let host = &rest[..rest.find('/').unwrap_or(rest.len())];
    if host.is_empty() {
        exit(format_args!("{R}Invalid host info.{N}"));
    }
    [scheme, host]
}

///Get `host` info and Generate `img/src/next/album` selector data
fn host_info(host: &str) -> [String; 4] {
    let data = website();
    let site = data
        .members()
        .find(|&s| {
            s["Site"]
                .as_str()
                .unwrap()
                .split_terminator(',')
                .any(|s| s == host.trim_start_matches("www."))
        })
        .unwrap_or_else(|| {
            exit(format_args!("Unsupported website.. {B}{R}🌏 {host} 💥{N}"));
        });
    let next = site["Next"].as_str().unwrap_or("");
    let album = site["Album"].as_str().unwrap_or("");
    [
        site["Img"].as_str().unwrap().to_owned(),
        site["Src"].as_str().unwrap().to_owned(),
        next.to_string(),
        album.to_owned(),
    ]
}

///Fetch web page generate html content
fn get_html(addr: &str) -> (String, [String; 4], [&str; 2]) {
    let scheme_host @ [_, host] = check_host(addr);
    let host_info = host_info(host);
    println!("{MARK}{BG}Downloading 📄 ...{N}");
    let out = process::Command::new("curl")
        .args(&[addr, "-e", host, "-A", "Mozilla Firefox", "-s", "-L"])
        .output()
        .unwrap_or_else(|e| {
            exit(format_args!("{R} `{e}` {N}"));
        });
    let res = String::from_utf8_lossy(&out.stdout).to_string();
    if res.is_empty() {
        exit(format_args!(
            "{R}Get HTML failed, please check url address.{N}"
        ));
    }
    (res, host_info, scheme_host)
}

///Parse photos in web url
fn parse(addr: &str) -> String {
    let (html, [img, src, mut next, album], [scheme, host]) = get_html(addr);
    let page = crabquery::Document::from(html);
    let imgs = page.select(img.as_str());
    let titles = page.select("title");
    let title = titles
        .first()
        .unwrap_or_else(|| {
            exit(format_args!("{R}Not a valid HTML page.{N}"));
        })
        .text()
        .expect("NO title text.");
    let mut t = title.trim();

    (0..2).for_each(|_| {
        t = t[..t.rfind(['/', '-', '_', '|', '–']).unwrap_or(t.len())].trim();
    });

    let slash2dot = t.replace('/', "·");
    t = slash2dot.as_ref();

    let albums = if album.is_empty() {
        vec![]
    } else {
        page.select(album.as_str())
    };
    let has_album = !album.is_empty() && !albums.is_empty();
    match (has_album, !imgs.is_empty()) {
        (true, true) => println!(
            "{B}Totally found {} 📸 and {} 🏞️  in 📄:{G} {}{N}",
            albums.len(),
            imgs.len(),
            t
        ),
        (true, false) => println!("{B}Totally found {} 📸 in 📄:{G} {}{N}", albums.len(), t),
        (false, true) => println!("{B}Totally found {} 🏞️  in 📄:{G} {}{N}", imgs.len(), t),
        (false, false) => {
            exit(format_args!("{B}∅ 🏞️  found in 📄:{G} {t}{N}"));
        }
    }

    #[cfg(any())]
    {
        if t.to_lowercase().contains("page") {
            t = t[..t.to_lowercase().rfind("page").unwrap()]
                .trim()
                .trim_end_matches(['-', '_', '|', '–', '/'])
                .trim();
        }
    }

    t = t[..t.rfind(['(', ',', '第']).unwrap_or(t.len())].trim();

    let canonicalize_url = |u: &str| {
        if !u.starts_with("http") {
            if u.starts_with("//") {
                format!("{scheme}:{u}")
            } else if u.starts_with('/') {
                format!("{scheme}://{host}{u}")
            } else {
                format!("{}/{u}", &addr[..addr.rfind('/').unwrap()])
            }
        } else {
            u.to_owned()
        }
    };
    match (has_album, !imgs.is_empty()) {
        (_, true) => {
            for img in imgs {
                let src = img.attr(src.as_str()).expect("Invalid img[src] selector!");
                let mut src = src.as_str();
                src = &src[src.rfind("?url=").map(|p| p + 5).unwrap_or(0)..];
                src = &src[..src.rfind('?').unwrap_or(src.len())];
                let file = canonicalize_url(src);
                //tdbg!(&file);
                download(t, &file);
            }
        }
        (true, false) => {
            let mut all = false;

            for (i, alb) in albums.iter().enumerate() {
                let mut parse_album = || {
                    let mut href = alb.attr("href").unwrap_or_else(|| {
                        alb.parent()
                            .unwrap()
                            .attr("href")
                            .expect("NO a[@href] attr found.")
                    });
                    let album_url = canonicalize_url(&href);
                    next = parse(&album_url);
                    while !next.is_empty() {
                        next = parse(&next);
                    }
                };

                if all {
                    parse_album();
                } else {
                    use io::*;
                    let mut stdin = io::stdin();
                    let mut stdout = io::stdout();
                    let mut t = alb.attr("title").unwrap_or(
                        alb.attr("alt")
                            .unwrap_or(alb.text().expect("NO Album's Title found.")),
                    );
                    writeln!(
                        stdout,
                        "{B}Do you want to download Album <{I}{U}{}/{}{N}>: {B}{G}{} ?{N}",
                        i + 1,
                        albums.len(),
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
                        exit(format_args!("{R} `{e}` {N}"));
                    });
                    input.make_ascii_lowercase();

                    match input.trim() {
                        "y" | "yes" | "" => parse_album(),
                        "n" | "no" => {
                            next = String::default();
                            continue;
                        }
                        "a" | "all" => {
                            all = true;
                            parse_album()
                        }
                        _ => {
                            println!("{B}Canceled all albums download.{N}");
                            next = String::default();
                            break;
                        }
                    };
                }
            }
        }
        (false, false) => (),
    }

    if (!next.is_empty()) {
        let nexts = page.select(&next);
        check_next(nexts, addr)
    } else {
        String::default()
    }
}

///Perform photo download operation
fn download(dir: &str, src: &str) {
    #[cfg(all(feature = "download", any(not(test), feature = "batch")))]
    {
        let path = path::Path::new(dir);
        if (!path.exists()) {
            fs::create_dir(path).unwrap_or_else(|e| {
                exit(format_args!("Create Dir error:{R} `{e}` {N}"));
            });
        }

        let name = src[src.rfind('/').unwrap() + 1..].trim_start_matches(['-', '_']);
        let host = &src[..src[10..].find('/').unwrap_or(src.len() - 10) + 10];
        let wget = format!("wget {src} -O {name} --referer={host} -U \"Mozilla Firefox\" -q");
        let curl = format!("curl {src} -o {name} -e {host} -A \"Mozilla Firefox\" -L -s");
        //tdbg!(&curl);
        if (path.exists() && !path.join(name).exists()) {
            #[cfg(feature = "curl")]
            process::Command::new("curl")
                .current_dir(path)
                .args(&[
                    src,
                    "-o",
                    name,
                    "-e",
                    host,
                    "-A",
                    "Mozilla Firefox",
                    "-L",
                    //"--location-trusted",
                    "-s",
                    #[cfg(feature = "retry")]
                    "--retry 3",
                ])
                .spawn();

            #[cfg(feature = "wget")]
            process::Command::new("wget")
                .current_dir(path)
                .args(&[
                    &src,
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
    let mut next: String;
    if nexts.is_empty() {
        next = String::default();
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

            next = a
                .first()
                .map_or(String::default(), |f| f.attr("href").unwrap());
        } else {
            next = nexts[0].attr("href").unwrap();
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
            next = s.first().map_or(String::default(), |f| {
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
            next = match last2 {
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
    if !next.is_empty() && !next.starts_with("http") {
        if next.trim() == "/" || next.trim() == "#" {
            next = String::default();
        } else {
            next = format!(
                "{}{}",
                if next.starts_with("//") {
                    &cur[..cur.find("//").unwrap()]
                } else if next.starts_with('/') {
                    &cur[..cur[10..].find('/').unwrap() + 10]
                } else {
                    &cur[..=cur.rfind('/').unwrap()]
                },
                next
            );
        }
    }

    tdbg!(next)
}

///WebSites `Json` config data
fn website() -> json::JsonValue {
    json::parse(include_str!("web.json")).unwrap_or_else(|e| {
        exit(format_args!("{R} `{e}` {N}"));
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html() {
        let addr = "mmm.red";
        let (html, ..) = get_html(addr);
        dbg!(&html);
    }

    #[test]
    fn try_it() {
        // https://bestgirlsexy.com https://girldreamy.com https://mmm.red

        let addr = "https://www.beautyleg6.com/siwameitui/";
        parse(addr);
    }

    #[test]
    fn htmlq() {
        let addr = "https://mmm.red";
        let (html, [img, .., album], _) = get_html(addr);
        use process::*;
        [img, album].iter().enumerate().for_each(|(i, sel)| {
            let cmd = Command::new("htmlq")
                .args(&[{
                    println!(
                        "\n{MARK}{B}{s} Selector: {HL} {sel} {N}",
                        s = if i == 0 { "Image" } else { "Album" }
                    );
                    sel
                }])
                .stdin(Stdio::piped())
                //.stdout(Stdio::piped())
                .spawn()
                .expect("Execute htmlq failed.");
            let mut stdin = cmd.stdin.as_ref().expect("Failed to open stdin.");
            use io::*;
            stdin.write_all(html.as_bytes()).expect("Failed to write.");
            cmd.wait_with_output().expect("Failed to get piped stdout.");
        });
    }

    #[test]
    fn run() {
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

    #[test]
    fn color() {
        let begin = time::Instant::now();
        color8(TEXT);
        // color256(TEXT);
        // color_rgb_fg_full();
        // color_rgb_bg_full();
        dbg!(&(time::Instant::now()-begin));
    }

}
