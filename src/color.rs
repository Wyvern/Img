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
    let text = "The quick brown fox jumps over the lazy dog";
    match (
        arg1.as_deref(),
        arg2.as_deref(),
        arg3.as_deref(),
        arg4.as_deref(),
    ) {
        (None, None, None, None) => {
            color8(text);
            color256(text)
        }
        (Some(v), None, None, None) if v.parse::<u8>() == Ok(8) => color8(text),
        (Some(v), None, None, None) if v.parse::<u16>() == Ok(256) => color256(text),
        (Some(v1), Some(v2), None, None) => match (v1.parse::<u16>(), v2.parse::<u8>()) {
            (Ok(256), Ok(c)) => {
                color_256_fg(&c, text);
                color_256_bg(&c, text);
            }
            (Ok(256), _) => match v2 {
                "f" | "fg" => color256_fg(text),
                "b" | "bg" => color256_bg(text),
                _ => exit(),
            },
            _ => exit(),
        },
        (Some("rgb") | Some("RGB"), Some(r), Some(g), Some(b)) => {
            let (r, g, b) = (r.parse::<u8>(), g.parse::<u8>(), b.parse::<u8>());
            if (r.is_ok() && g.is_ok() && b.is_ok()) {
                let (r, g, b) = (
                    r.as_ref().unwrap(),
                    g.as_ref().unwrap(),
                    b.as_ref().unwrap(),
                );
                color_rgb_fg(r, g, b, text);
                color_rgb_bg(r, g, b, text)
            } else {
                exit();
            }
        }
        _ => exit(),
    }
}

fn exit() {
    println!("Please input `8|256` `256 {B}<color>{N} [0-255]` `256 {{{B}{FG}f,{BG}b{N}}}[g]` or `RGB {B}{R}r {G}g {BLUE}b{N} [0-255]{{3}}` color options.");
    process::exit(0);
}
