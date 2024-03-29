use std::*;

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
mod color {
    pub static N: &str = "\x1b[0m";
    pub static B: &str = "\x1b[1m";
    pub static _B: &str = "\x1b[22m";
    pub static I: &str = "\x1b[3m";
    pub static _I: &str = "\x1b[23m";
    pub static U: &str = "\x1b[4m";
    pub static _U: &str = "\x1b[24m";
    pub static BEG: &str = "\x1b[G"; //Move to begin of line
    pub static CL: &str = "\x1b[2K"; //Erase the entire line
    pub static UU: &str = "\x1b[21m";
    pub static R: &str = "\x1b[91m";
    pub static G: &str = "\x1b[92m";
    pub static Y: &str = "\x1b[93m";
    pub static BLUE: &str = "\x1b[94m";
    pub static HL: &str = "\x1b[103m";
    pub static BG: &str = "\x1b[100m";
    pub static FG: &str = "\x1b[97m";
    pub static SAVE: &str = "\x1b[s"; //"\x1b7" save cursor & attrs
    pub static REST: &str = "\x1b[u"; //"\x1b8" unsave cursor & attrs
    pub static MARK: &str = "\x1b]1337;SetMark\x07";
    pub static TEXT: &str = "The quick brown fox jumps over the lazy dog";

    use std::io::*;

    pub fn color8(text: &str) {
        let mut bf = BufWriter::new(stdout());
        (0u8..10)
            .chain(21..=21)
            .chain(30..=37)
            .chain(40..=47)
            .chain(90..=97)
            .chain(100..=107)
            .for_each(|c| {
                match c {
                    0 => writeln!(bf, "\n{B}{U}Basic Style:{N}"),
                    30 => writeln!(bf, "\n{B}{U}8-color regular foreground:{N}"),
                    40 => writeln!(bf, "\n{B}{U}8-color regular background:{N}"),
                    90 => writeln!(bf, "\n{B}{U}8-color bright foreground:{N}"),
                    100 => writeln!(bf, "\n{B}{U}8-color bright background:{N}"),
                    _ => Ok(()),
                };
                writeln!(bf, "\"\\x1b[{c}m\": - \x1b[{c}m {text} {N}");
            });
        bf.flush();
    }

    pub fn color256(text: &str) {
        color256_fg(text);
        color256_bg(text);
    }

    pub fn color256_fg(text: &str) {
        let mut bf = BufWriter::new(stdout());
        writeln!(bf, "\n{B}{U}256-color foreground:{N}");
        (0u8..=255).for_each(|c| {
            writeln!(bf, "\"\\x1b[38;5;{c}m\": - \x1b[38;5;{c}m {text} {N}");
        });
        bf.flush();
    }

    pub fn color256_bg(text: &str) {
        let mut bf = BufWriter::new(stdout());
        writeln!(bf, "\n{B}{U}256-color background:{N}");
        (0u8..=255).for_each(|c| {
            writeln!(bf, "\"\\x1b[48;5;{c}m\": - \x1b[48;5;{c}m {text} {N}");
        });
        bf.flush();
    }

    pub fn color_256_fg(c: u8, text: &str) {
        let mut bf = BufWriter::new(stdout());
        writeln!(bf, "\n{B}{U}256-color foreground:{N}");
        writeln!(bf, "\"\\x1b[38;5;{c}m\": - \x1b[38;5;{c}m {text} {N}");
        bf.flush();
    }

    pub fn color_256_bg(c: u8, text: &str) {
        let mut bf = BufWriter::new(stdout());
        writeln!(bf, "\n{B}{U}256-color background:{N}");
        writeln!(bf, "\"\\x1b[48;5;{c}m\": - \x1b[48;5;{c}m {text} {N}");
        bf.flush();
    }

    pub fn color_rgb_fg(rgb: [u8; 3], text: &str) {
        let mut bf = BufWriter::new(stdout());
        writeln!(bf, "\n{B}{U}RGB-color foreground:{N}");
        writeln!(
            bf,
            "\"\\x1b[38;2;{0};{1};{2}m\": - \x1b[38;2;{0};{1};{2}m {text} {N}",
            rgb[0], rgb[1], rgb[2]
        );
        bf.flush();
    }

    pub fn color_rgb_bg(rgb: [u8; 3], text: &str) {
        let mut bf = BufWriter::new(stdout());
        writeln!(bf, "\n{B}{U}RGB-color background:{N}");
        writeln!(
            bf,
            "\"\\x1b[48;2;{0};{1};{2}m\": - \x1b[48;2;{0};{1};{2}m {text} {N}",
            rgb[0], rgb[1], rgb[2]
        );
        bf.flush();
    }

    pub fn color_rgb_fg_full() {
        let mut bf = BufWriter::new(stdout());
        (0u8..=255).for_each(|r| {
            (0u8..=255).for_each(|g| {
                (0u8..=255).for_each(|b| {
                    writeln!(bf,"\"\\x1b[38;2;{r};{g};{b}m\": - \x1b[38;2;{r};{g};{b}m Full-range foreground RGB-color {N}");
                });bf.flush();super::pause("")
            });
        });
    }

    pub fn color_rgb_bg_full() {
        let mut bf = BufWriter::new(stdout());
        (0u8..=255).for_each(|r| {
            (0u8..=255).for_each(|g| {
                (0u8..=255).for_each(|b| {
                    writeln!(bf,"\"\\x1b[48;2;{r};{g};{b}m\": - \x1b[48;2;{r};{g};{b}m Full-range background RGB-color {N}");
                });bf.flush();super::pause("")
            })
        });
    }
}
pub use color::*;

mod macros {

    #[macro_export]
    macro_rules! quit {
        ($l:literal $(,$e:expr)*) => {{
            pl!($l $(,$e)*);
            process::exit(0);
        }}
    }

    #[macro_export]
    macro_rules! pl {
        ($l:literal $(,$e:expr)*) => {{
            println!("{B}{}{N}", format_args!($l $(,format_args!("`{R}{}{N}{B}`",$e))*));
        }}
    }

    #[macro_export]
    macro_rules! tdbg {
        ($($e:expr),*) => {
            if cfg!(test) || cfg!(debug_assertions) {
                io::stdout().lock();
                let r=$crate::dbg!(($($e),*));
                #[cfg(test)]{pause("");}
                r
            } else {($($e),*)}
        }
    }

    macro_rules! demo {
    ([$attr:meta ] $pub:vis & $lt:lifetime $name:ident : $type:ty = $l:literal | $e:expr, $s:stmt ; $pat:pat => $b:block | $p:path | $i:item | $t:tt) => {$pat $t};

    ($id:ident, $b:block, $stmt:stmt, $e:expr, $pat:pat, $t:ty, $lt:lifetime, $l:literal, $p:path, $m:meta, $tt:tt, $i:item, $v:vis)=>{};

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
}

pub fn pause(msg: &str) {
    use io::*;
    let mut o = stdout().lock();
    write!(
        o,
        "{}",
        if msg.is_empty() {
            "Press any key to continue..."
        } else {
            msg
        }
    );
    o.flush();
    stdin().lock().read_line(&mut String::default());
}

pub fn dyn_value<T>(mut var: &dyn any::Any, val: T) {
    let ptr = var as *const _ as *mut _;
    let cell = cell::Cell::new(ptr);
    unsafe {
        *cell.get() = val;
    }
}

pub fn dyn_cast<T: Copy>(mut var: &dyn any::Any) -> T {
    let ptr = var as *const _ as *const _;
    unsafe { *ptr }
}

const fn is_target_little_endian() -> bool {
    u16::from_ne_bytes([1, 0]) == 1
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;

    #[test]
    fn dyn_any() {
        tdbg!(is_target_little_endian());
        let x = [&mut 7 as &dyn any::Any, &4.3];

        let y = 123;
        dbg!(y, dyn_cast::<char>(&y));
        dyn_value(&y, 456);
        dbg!(y, dyn_cast::<char>(&y));

        dyn_value(x[0], "rust");
        dbg!(dyn_cast::<&str>(x[0]));
        dyn_value(x[0], -123);
        dbg!(dyn_cast::<u8>(x[0]));

        dbg!(dyn_cast::<f32>(x[1]));
        dbg!(dyn_cast::<char>(x[1]));
        dbg!(dyn_cast::<f64>(x[1]));

        let mut z = 111;
        dbg!(&mem::replace(&mut z, 128));
        dbg!(&z);
    }
}
