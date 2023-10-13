use std::*;

use crabquery as _;
use json as _;

mod util;
use util::*;

fn main() {
    if env::args().len() > 5 {
        exit()
    }
    let args: [_; 4] = array::from_fn(|i| env::args().nth(i + 1));

    analyze_args(array::from_fn(|i| args[i].as_deref()));
}

fn analyze_args(args: [Option<&str>; 4]) {
    match args {
        [None, None, None, None] => {
            color8(TEXT);
            color256(TEXT)
        }
        [Some(v), None, None, None] if v.parse::<u8>() == Ok(8) => color8(TEXT),
        [Some(v), None, None, None] if v.parse::<u16>() == Ok(256) => color256(TEXT),
        [Some(v1), Some(v2), None, None] => match (
            v1.parse::<u16>().as_ref(),
            v2.parse::<u8>()
                .or_else(|_| u8::from_str_radix(v2.strip_prefix("0x").unwrap_or(v2), 16)),
        ) {
            (Ok(256), Ok(c)) => {
                color_256_fg(c, TEXT);
                color_256_bg(c, TEXT);
            }
            (Ok(256), _) => match v2 {
                "fg" => color256_fg(TEXT),
                "bg" => color256_bg(TEXT),
                _ => exit(),
            },
            _ => exit(),
        },
        [Some("rgb") | Some("RGB"), Some(r), Some(g), Some(b)] => {
            match [r, g, b].map(|v| {
                v.parse::<u8>()
                    .or_else(|_| u8::from_str_radix(v.trim_start_matches("0x"), 16))
            }) {
                [Ok(r), Ok(g), Ok(b)] => {
                    color_rgb_fg([r, g, b], TEXT);
                    color_rgb_bg([r, g, b], TEXT);
                }
                _ => exit(),
            }
        }
        _ => exit(),
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
    fn run() {
        let mut s = "256 ab ".split_whitespace();
        let args = array::from_fn(|_| s.next());
        analyze_args(args);
    }

    #[test]
    fn show() {
        let begin = time::Instant::now();
        color8(TEXT);
        // color256(TEXT);
        // color_rgb_fg_full();
        // color_rgb_bg_full();
        dbg!(&(time::Instant::now() - begin));
    }
}
