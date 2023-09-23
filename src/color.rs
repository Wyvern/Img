use std::*;

use crabquery as _;
use json as _;

mod util;
use util::*;

fn main() {
    let (arg1, arg2, arg3, arg4) = (
        env::args().nth(1),
        env::args().nth(2),
        env::args().nth(3),
        env::args().nth(4),
    );
    if env::args().len() > 5 {
        exit()
    }
    let args = if cfg!(test) {
        (Some("rgb"), Some("ab"), Some("cd"), Some("ef"))
    } else {
        (
            arg1.as_deref(),
            arg2.as_deref(),
            arg3.as_deref(),
            arg4.as_deref(),
        )
    };
    match args {
        (None, None, None, None) => {
            color8(TEXT);
            color256(TEXT)
        }
        (Some(v), None, None, None) if v.parse::<u8>() == Ok(8) => color8(TEXT),
        (Some(v), None, None, None) if v.parse::<u16>() == Ok(256) => color256(TEXT),
        (Some(v1), Some(v2), None, None) => match (
            v1.parse::<u16>(),
            v2.parse::<u8>()
                .or_else(|_| u8::from_str_radix(v2.trim_start_matches("0x"), 16)),
        ) {
            (Ok(256), Ok(c)) => {
                color_256_fg(&c, TEXT);
                color_256_bg(&c, TEXT);
            }
            (Ok(256), _) => match v2 {
                "f" | "fg" => color256_fg(TEXT),
                "b" | "bg" => color256_bg(TEXT),
                _ => exit(),
            },
            _ => exit(),
        },
        (Some("rgb") | Some("RGB"), Some(r), Some(g), Some(b)) => {
            let dec_or_hex = |v: &str| {
                v.parse::<u8>()
                    .or_else(|_| u8::from_str_radix(v.trim_start_matches("0x"), 16))
            };
            let (r, g, b) = (dec_or_hex(r), dec_or_hex(g), dec_or_hex(b));
            match (r, g, b) {
                (Ok(ref r), Ok(ref g), Ok(ref b)) => {
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
    println!("Please input `8|256` `256 {B}<color>{N} [0-255]` `256 {{{B}{FG}f,{BG}b{N}}}[g]` or `RGB {B}{R}r {G}g {BLUE}b{N} [0-255]{{3}}` color options.");
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
