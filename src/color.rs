use std::*;

use crabquery as _;
use json as _;

mod util;
use util::*;

fn main() {
    let arg1 = env::args().nth(1);
    let arg2 = env::args().nth(2);
    let text = "The quick brown fox jumps over the lazy dog";
    let msg = "Please input `8/256` `f[g]/b[g]` color options.";
    match (arg1, arg2) {
        (None, None) => {
            color8(text);
            color256(text)
        }
        (Some(v), None) if v.parse::<u16>() == Ok(8) => color8(text),
        (Some(v), None) if v.parse::<u16>() == Ok(256) => color256(text),
        (Some(v1), Some(v2)) => match (v1.parse::<u16>(), v2.parse::<u16>()) {
            (Ok(8), Ok(256)) => {
                color8(text);
                color256(text);
            }
            (Ok(256), Ok(8)) => {
                color256(text);
                color8(text);
            }
            (Ok(256), _) => match v2.as_str() {
                "f" | "fg" => color256_fg(text),
                "b" | "bg" => color256_bg(text),
                _ => exit(format_args!("{msg}")),
            },
            _ => exit(format_args!("{msg}")),
        },
        _ => exit(format_args!("{msg}")),
    }
}