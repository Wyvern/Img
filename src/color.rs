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
    let _args = if cfg!(test) {
        let mut s = "256 ab".split_whitespace();
        array::from_fn(|_| s.next())
    } else {
        array::from_fn(|i| args[i].as_deref())
    };

    match _args {
        [None, None, None, None] => {
            color8(TEXT);
            color256(TEXT)
        }
        [Some(v), None, None, None] if v.parse::<u8>() == Ok(8) => color8(TEXT),
        [Some(v), None, None, None] if v.parse::<u16>() == Ok(256) => color256(TEXT),
        [Some(v1), Some(v2), None, None] => match (
            v1.parse::<u16>().as_ref(),
            v2.parse::<u8>()
                .or_else(|_| u8::from_str_radix(v2.trim_start_matches("0x"), 16))
                .as_ref(),
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
            match [r, g, b]
                .map(|v| {
                    v.parse::<u8>()
                        .or_else(|_| u8::from_str_radix(v.trim_start_matches("0x"), 16))
                })
                .as_ref()
            {
                [Ok(r), Ok(g), Ok(b)] => {
                    color_rgb_fg(r, g, b, TEXT);
                    color_rgb_bg(r, g, b, TEXT);
                }
                _ => exit(),
            }
        }
        _ => exit(),
    }
}

fn exit() {
    println!("Please input `8|256` `256 {B}<color>{N} [0-255]` `256 {{{B}{FG}fg,{BG}bg{N}}}` or `RGB {B}{R}r {G}g {BLUE}b{N} [0-255]{{3}}` color options.");
    process::exit(0);
}

#[cfg(test)]
mod color {
    use super::*;

    #[test]
    fn run() {
        main()
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
