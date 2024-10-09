use {std::*, util::*};

mod util;

fn main() -> io::Result<()> {
    if env::args().len() > if cfg!(test) { 5 + 3 } else { 5 } {
        exit()
    }

    let args: [_; 4] = array::from_fn(|i| {
        if cfg!(test) {
            env::args().skip(3).nth(i + 1)
        } else {
            env::args().nth(i + 1)
        }
    });

    analyze_args(array::from_fn(|i| args[i].as_deref()))
}

fn analyze_args(args: [Option<&str>; 4]) -> io::Result<()> {
    match args {
        [None, None, None, None] => {
            color8(TEXT)?;
            color(Range::_256(0), TEXT, Kind::Both, true)
        }
        [Some(v), None, None, None] if v.parse::<u8>() == Ok(8) => color8(TEXT),
        [Some(v), None, None, None] if v.parse::<u16>() == Ok(256) => {
            color(Range::_256(0), TEXT, Kind::Both, true)
        }
        [Some(v1), Some(v2), None, None] => match (
            v1.parse::<u16>().as_ref(),
            v2.parse::<u8>()
                .or_else(|_| u8::from_str_radix(v2.strip_prefix("0x").unwrap_or(v2), 16)),
        ) {
            (Ok(256), Ok(c)) => {
                color(Range::_256(c), TEXT, Kind::FG, false)?;
                color(Range::_256(c), TEXT, Kind::BG, false)
            }
            (Ok(256), _) => match v2 {
                "fg" => color(Range::_256(0), TEXT, Kind::FG, true),
                "bg" => color(Range::_256(0), TEXT, Kind::BG, true),
                _ => {
                    exit();
                    Ok(())
                }
            },
            _ => {
                exit();
                Ok(())
            }
        },
        [Some("rgb") | Some("RGB"), Some(r), Some(g), Some(b)] => {
            match [r, g, b].map(|v| {
                v.parse::<u8>()
                    .or_else(|_| u8::from_str_radix(v.trim_start_matches("0x"), 16))
            }) {
                [Ok(r), Ok(g), Ok(b)] => {
                    color(Range::_RGB(r, g, b), TEXT, Kind::FG, false)?;
                    color(Range::_RGB(r, g, b), TEXT, Kind::BG, false)
                }
                _ => {
                    exit();
                    Ok(())
                }
            }
        }
        _ => {
            exit();
            Ok(())
        }
    }
}

fn exit() {
    println!("Usage: Color {NL}`8|256` {NL}`256 {B}<color>{N} [0-255]` {NL}`256 {{{B}{FG}fg,{BG}bg{N}}}` {NL}`RGB {B}{R}r {G}g {BLUE}b{N} [0-255]{{3}}` \n options.",NL="\n\t");
    process::exit(0);
}

#[cfg(test)]
mod color {
    use super::*;

    #[test]
    fn run() -> io::Result<()> {
        main()
    }

    #[test]
    fn show() -> io::Result<()> {
        color8(TEXT)?;
        color(Range::_256(0), TEXT, Kind::Both, true)?;
        Ok(())
    }
}

use io::*;

fn color8(text: &str) -> Result<()> {
    let mut bf = BufWriter::new(stdout());
    (0u8..10)
        .chain(21..=21)
        .chain(30..=37)
        .chain(40..=47)
        .chain(90..=97)
        .chain(100..=107)
        .for_each(|c| {
            _ = match c {
                0 => writeln!(bf, "\n{B}{U}Basic Style:{N}"),
                30 => writeln!(bf, "\n{B}{U}8-color regular foreground:{N}"),
                40 => writeln!(bf, "\n{B}{U}8-color regular background:{N}"),
                90 => writeln!(bf, "\n{B}{U}8-color bright foreground:{N}"),
                100 => writeln!(bf, "\n{B}{U}8-color bright background:{N}"),
                _ => Ok(()),
            };
            _ = writeln!(bf, "\"\\x1b[{c}m\": - \x1b[{c}m {text} {N}");
        });
    bf.flush()
}

enum Range {
    _256(u8),
    _RGB(u8, u8, u8),
}

enum Kind {
    FG,
    BG,
    Both,
}

fn color(r: Range, text: &str, k: Kind, full: bool) -> Result<()> {
    let mut bf = BufWriter::new(stdout());
    match k {
        Kind::Both => {
            color(Range::_256(0), text, Kind::FG, true)?;
            color(Range::_256(0), text, Kind::BG, true)?;
            return Ok(());
        }
        _ => writeln!(
            bf,
            "\n{B}{U}{}-color {}:{N}",
            match r {
                Range::_256(_) => "256",
                Range::_RGB(..) => "RGB",
            },
            match k {
                Kind::FG => "foreground",
                Kind::BG => "background",
                Kind::Both => unreachable!(),
            }
        )?,
    }

    let fb: u8 = match k {
        Kind::FG => 38,
        Kind::BG => 48,
        _ => unreachable!(),
    };

    if full {
        match r {
            Range::_256(_) => (0u8..=255).for_each(|c| {
                _ = writeln!(bf, "\"\\x1b[{fb};5;{c}m\": - \x1b[{fb};5;{c}m {text} {N}");
            }),
            Range::_RGB(..) => (0u8..=255).for_each(|r| {
                (0u8..=255).for_each(|g| {
                    (0u8..=255).for_each(|b| {
                        _ = writeln!(
                            bf,
                            "\"\\x1b[{fb};2;{r};{g};{b}m\": - \x1b[{fb};2;{r};{g};{b}m {text} {N}"
                        );
                    });
                    _ = bf.flush();
                    pause("")
                });
            }),
        }
    } else {
        match r {
            Range::_256(c) => writeln!(bf, "\"\\x1b[{fb};5;{c}m\": - \x1b[{fb};5;{c}m {text} {N}")?,
            Range::_RGB(r, g, b) => writeln!(
                bf,
                "\"\\x1b[{fb};2;{r};{g};{b}m\": - \x1b[{fb};2;{r};{g};{b}m {text} {N}"
            )?,
        }
    }

    bf.flush()
}
