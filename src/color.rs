use std::*;
use json as _;
use crabquery as _;

mod Color {
    pub static N: &str = "\x1b[0m";

}

use Color::*;

fn main(){
    let text="The quick brown fox jumps over the lazy dog";
        (0u8..10)
            .chain((21..22))
            .chain((30..=37))
            .chain((40..=47))
            .for_each(|c| println!("\"\\x1b[{c}m\": - \x1b[{c}m {text} {N}"));

        (0u8..=255).for_each(|c| println!("\"\\x1b[38;5;{c}m\": - \x1b[38;5;{c}m {text} {N}"));

        (0u8..=255).for_each(|c| println!("\"\\x1b[48;5;{c}m\": - \x1b[48;5;{c}m {text} {N}"));
}