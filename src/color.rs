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
        color8(TEXT)
        // color256(TEXT);
        // color_rgb_fg_full();
        // color_rgb_bg_full();
    }
}
