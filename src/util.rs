use std::{fmt::*, *};

///Output dbg!(expr) only in cfg!(test) || cfg!(debug_assertions) context
fn tdbg<T: Debug>(expr: T) -> T {
    if cfg!(test) || cfg!(debug_assertions) {
        dbg!(expr)
    } else {
        expr
    }
}

//#[macro_export]
macro_rules! demo {
    ([$attr:meta ] $pub:vis & $lt:lifetime $name:ident : $type:ty = $l:literal | $e:expr, $s:stmt ; $pat:pat_param => $b:block | $p:path | $i:item | $t:tt ) => {$pat:pat $pat:pat_param};
}

#[macro_export]
macro_rules! tdbg {
    ($e:expr $(,)?) => {
        if cfg!(test) || cfg!(debug_assertions) {
            dbg!($e)
        } else {
            $e
        }
    };
    ($($e:expr),+ $(,)?) => {
        if cfg!(test) || cfg!(debug_assertions) {
            ($($crate::dbg!($e)),+,)
        } else {
            ($($e),+,)
        }
    };
}

macro_rules! impl_ref_elements {
    () => {};
    ($T0:ident $($T:ident)*) => {
        impl<$T0, $($T,)*> RefElements for ($T0,$($T,)*) {
            type Refs<'a> = (&'a $T0, $(&'a $T,)*) where Self:'a;
            fn ref_elements(&self)->Self::Refs<'_> {
                let &(ref $T0,$(ref $T,)*) = self;
                ($T0,$($T,)*)
            }
        }
        impl_ref_elements!{$($T)*}
    }
}

///Colorized terminal constants
/**
    The 8 actual colors within the ranges (30-37, 40-47, 90-97, 100-107) are defined by the ANSI standard as follows:
    Last Digit 	Color
    0 	black
    1 	red
    2 	green
    3 	yellow
    4 	blue
    5 	magenta
    6 	cyan
    7 	white

    Some common SGR parameters are shown below:
    Parameter 	Effect
    0 	reset all SGR effects to their default
    1 	bold or increased intensity
    2 	faint or decreased intensity
    4 	singly underlined
    5 	slow blink
    30-37 	foreground color (8 colors)
    38;5;x 	foreground color (256 colors, non-standard)
    38;2;r;g;b 	foreground color (RGB, non-standard)
    40-47 	background color (8 colors)
    48;5;x 	background color (256 colors, non-standard)
    48;2;r;g;b 	background color (RGB, non-standard)
    90-97 	bright foreground color (non-standard)
    100-107 	bright background color (non-standard)
*/
mod Color {
    pub static N: &str = "\x1b[0m";
    pub static B: &str = "\x1b[1m";
    pub static I: &str = "\x1b[3m";
    pub static U: &str = "\x1b[4m";
    pub static UU: &str = "\x1b[21m";
    pub static R: &str = "\x1b[91m";
    pub static G: &str = "\x1b[92m";
    pub static Y: &str = "\x1b[93m";
    pub static HL: &str = "\x1b[103m";
    pub static BG: &str = "\x1b[100m";
    pub static MARK: &str = "\x1b]1337;SetMark\x07";

    pub fn color8(text: &str) {
        (0u8..10)
            .chain((21..22))
            .chain((30..=37))
            .chain((40..=47))
            .chain((90..=97))
            .chain((100..=107))
            .for_each(|c| {
                match c {
                    0 => println!("\n{B}{U}Basic Style:{N}"),
                    30 => println!("\n{B}{U}8-color regular foreground:{N}"),
                    40 => println!("\n{B}{U}8-color regular background:{N}"),
                    90 => println!("\n{B}{U}8-color bright foreground:{N}"),
                    100 => println!("\n{B}{U}8-color bright background:{N}"),
                    _ => (),
                }
                println!("\"\\x1b[{c}m\": - \x1b[{c}m {text} {N}");
            });
    }

    pub fn color256(text: &str) {
        color256_fg(text);
        color256_bg(text);
    }

    pub fn color256_fg(text: &str) {
        println!("\n{B}{U}256-color foreground:{N}");
        (0u8..=255).for_each(|c| println!("\"\\x1b[38;5;{c}m\": - \x1b[38;5;{c}m {text} {N}"));
    }

    pub fn color256_bg(text: &str) {
        println!("\n{B}{U}256-color background:{N}");
        (0u8..=255).for_each(|c| println!("\"\\x1b[48;5;{c}m\": - \x1b[48;5;{c}m {text} {N}"));
    }

    pub fn color_rgb_fg(r: &u8, g: &u8, b: &u8, text: &str) {
        println!("\n{B}{U}RGB-color foreground:{N}");
        println!("\"\\x1b[38;2;{r};{g};{b}m\": - \x1b[38;2;{r};{g};{b}m {text} {N}");
    }

    pub fn color_rgb_bg(r: &u8, g: &u8, b: &u8, text: &str) {
        println!("\n{B}{U}RGB-color background:{N}");
        println!("\"\\x1b[48;2;{r};{g};{b}m\": - \x1b[48;2;{r};{g};{b}m {text} {N}");
    }

    pub fn color_rgb_fg_full() {
        (0u8..=255).for_each(|r| {
            (0u8..=255).for_each(|g| {
                (0u8..=255).for_each(|b| {
                    println!("\"\\x1b[38;2;{r};{g};{b}m\": - \x1b[38;2;{r};{g};{b}m Full-range foreground RGB-color {N}");
                });super::pause()
            });
        });
    }

    pub fn color_rgb_bg_full() {
        (0u8..=255).for_each(|r| {
            (0u8..=255).for_each(|g| {
                (0u8..=255).for_each(|b| {
                    println!("\"\\x1b[48;2;{r};{g};{b}m\": - \x1b[48;2;{r};{g};{b}m Full-range background RGB-color {N}");
                });super::pause()
            })
        });
    }
}
pub use Color::*;

///Generic `exit(msg: ...)` function
pub fn exit(msg: Arguments<'_>) -> ! {
    println!("{msg}");
    process::exit(0);
}

// #[test]
fn pause() {
    use io::*;
    let mut input = io::stdin();
    let mut output = io::stdout();

    write!(output, "Press any key to continue...");
    output.flush();

    let mut handle = input.lock();
    handle.read_line(&mut String::default());
}
