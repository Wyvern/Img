use std::*;

use crabquery as _;
use json as _;

mod Color {
    pub static N: &str = "\x1b[0m";
    pub static B: &str = "\x1b[1m";
    pub static U: &str = "\x1b[4m";
}

use Color::*;

fn main() {
    let arg1 = env::args().nth(1);
    let arg2 = env::args().nth(2);
    let text = "The quick brown fox jumps over the lazy dog";

    match (arg1, arg2) {
        (None, None) => {
            _8color(text);
            _256color(text)
        }
        (Some(v), None) if v.parse::<u16>() == Ok(8) => _8color(text),

        (Some(v), None) if v.parse::<u16>() == Ok(256) => _256color(text),

        (Some(v1), Some(v2)) => match (v1.parse::<u16>(), v2.parse::<u16>()) {
            (Ok(8), Ok(8)) => _8color(text),
            (Ok(8), Ok(256)) => {
                _8color(text);
                _256color(text);
            }
            (Ok(256), Ok(8)) => {
                _256color(text);
                _8color(text);
            }
            (Ok(256), Ok(256)) => _256color(text),
            _ => {
                println!("Please input 8/256 color options.");
                process::exit(0);
            }
        },
        _ => {
            println!("Please input 8/256 color options.");
            process::exit(0);
        }
    }
}

fn _8color(text: &str) {
    (0u8..10)
        .chain((21..22))
        .chain((30..=37))
        .chain((40..=47))
        .chain((90..=97))
        .chain((100..=107))
        .for_each(|c| {
            match c {
                0 => println!("{B}{U}Basic Style:{N}"),
                30 => println!("{B}{U}\n8-color regular foreground:{N}"),
                40 => println!("{B}{U}\n8-color regular background:{N}"),
                90 => println!("{B}{U}\n8-color bright foreground:{N}"),
                100 => println!("{B}{U}\n8-color bright background:{N}"),
                _ => (),
            }
            println!("\"\\x1b[{c}m\": - \x1b[{c}m {text} {N}")
        });
}

fn _256color(text: &str) {
    #[cfg(all())]
    {
        println!("{B}{U}\n256-color foreground:{N}");
        (0u8..=255).for_each(|c| println!("\"\\x1b[38;5;{c}m\": - \x1b[38;5;{c}m {text} {N}"));

        println!("{B}{U}\n256-color background:{N}");
        (0u8..=255).for_each(|c| println!("\"\\x1b[48;5;{c}m\": - \x1b[48;5;{c}m {text} {N}"));
    }
}
