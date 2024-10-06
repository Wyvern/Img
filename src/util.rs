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
    macro_rules! color {
            ($($i:ident = $l:literal),+) => {
                $(pub static $i: &str = $l;)+
            };
        }

    color!(
        N = "\x1b[0m",
        B = "\x1b[1m",
        _B = "\x1b[22m",
        I = "\x1b[3m",
        _I = "\x1b[23m",
        U = "\x1b[4m",
        _U = "\x1b[24m",
        BEG = "\x1b[G", //Move to begin of line
        CL = "\x1b[2K", //Erase the entire line
        UU = "\x1b[21m",
        R = "\x1b[91m",
        G = "\x1b[92m",
        Y = "\x1b[93m",
        BLUE = "\x1b[94m",
        HL = "\x1b[103m",
        BG = "\x1b[100m",
        FG = "\x1b[97m",
        SAVE = "\x1b[s", //"\x1b7" save cursor & attrs
        REST = "\x1b[u", //"\x1b8" unsave cursor & attrs
        MARK = "\x1b]1337;SetMark\x07",
        TEXT = "The quick brown fox jumps over the lazy dog"
    );

    use std::io::*;

    pub fn color8(text: &str) -> Result<()> {
        let mut bf = BufWriter::new(stdout());
        (0u8..10)
            .chain(21..=21)
            .chain(30..=37)
            .chain(40..=47)
            .chain(90..=97)
            .chain(100..=107)
            .for_each(|c| {
                let _ = match c {
                    0 => writeln!(bf, "\n{B}{U}Basic Style:{N}"),
                    30 => writeln!(bf, "\n{B}{U}8-color regular foreground:{N}"),
                    40 => writeln!(bf, "\n{B}{U}8-color regular background:{N}"),
                    90 => writeln!(bf, "\n{B}{U}8-color bright foreground:{N}"),
                    100 => writeln!(bf, "\n{B}{U}8-color bright background:{N}"),
                    _ => Ok(()),
                };
                let _ = writeln!(bf, "\"\\x1b[{c}m\": - \x1b[{c}m {text} {N}");
            });
        bf.flush()
    }

    pub enum Range {
        _256(u8),
        _RGB(u8, u8, u8),
    }

    pub enum Kind {
        FG,
        BG,
        Both,
    }

    pub fn color(r: Range, text: &str, k: Kind, full: bool) -> Result<()> {
        let mut bf = BufWriter::new(stdout());
        match k {
            Kind::Both => {
                color(Range::_256(0), text, Kind::FG, true)?;
                color(Range::_256(0), text, Kind::BG, true)?;
                return Ok(());
            }
            Kind::FG => writeln!(
                bf,
                "\n{B}{U}{}-color foreground:{N}",
                match r {
                    Range::_256(_) => "256",
                    Range::_RGB(..) => "RGB",
                }
            )?,
            Kind::BG => writeln!(
                bf,
                "\n{B}{U}{}-color background:{N}",
                match r {
                    Range::_256(_) => "256",
                    Range::_RGB(..) => "RGB",
                }
            )?,
        }

        let fb: u8 = match k {
            Kind::FG => 38,
            Kind::BG => 48,
            _ => unreachable!(),
        };

        if full {
            match r {
                Range::_256(_) => (0u8..=255).for_each(|c| {
                    let _ = writeln!(
                        bf,
                        "\"\\x1b[{fb};5;{c}m\": - \x1b[{fb};5;{c}m {text} {N}"
                    );
                }),
                Range::_RGB(..) => (0u8..=255).for_each(|r| {
                    (0u8..=255).for_each(|g| {
                        (0u8..=255).for_each(|b| {
                            let _=writeln!(bf,"\"\\x1b[{fb};2;{r};{g};{b}m\": - \x1b[{fb};2;{r};{g};{b}m {text} {N}");
                        });let _=bf.flush();super::pause("")
                    });
                }),
            }
        } else {
            match r {
                Range::_256(c) => {
                    writeln!(bf, "\"\\x1b[{fb};5;{c}m\": - \x1b[{fb};5;{c}m {text} {N}")?
                }
                Range::_RGB(r, g, b) => writeln!(
                    bf,
                    "\"\\x1b[{fb};2;{r};{g};{b}m\": - \x1b[{fb};2;{r};{g};{b}m {text} {N}"
                )?,
            }
        }

        bf.flush()
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
    macro_rules! p {
        ($l:literal $(,$e:expr)*) => {{
            print!("{B}{}{N}", format_args!($l $(,format_args!("`{R}{}{N}{B}`",$e))*));
        }}
    }

    #[macro_export]
    macro_rules! tdbg {
        ($($e:expr),*) => {
            if cfg!(test) || cfg!(debug_assertions) {
                let _ = io::stdout().lock();
                let r = dbg!(($($e),*));
                #[cfg(test)]{pause("")}
                r
            } else {($($e),*)}
        }
    }

    macro_rules! _demo {
    ([$attr:meta ] $pub:vis & $lt:lifetime $name:ident : $type:ty = $l:literal | $e:expr, $s:stmt ; $pat:pat => $b:block | $p:path | $i:item | $t:tt) => {$pat $t};

    ($id:ident, $b:block, $stmt:stmt, $e:expr, $pat:pat, $t:ty, $lt:lifetime, $l:literal, $p:path, $m:meta, $tt:tt, $i:item, $v:vis)=>{};

    }

    macro_rules! _impl_ref_elements {
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
    let _ = write!(
        o,
        "{}",
        if msg.is_empty() {
            "Press any key to continue..."
        } else {
            msg
        }
    );
    let _ = o.flush();
    let _ = stdin().lock().read_line(&mut String::default());
}

fn dyn_set<T>(var: &dyn any::Any, val: T) {
    let ptr = var as *const _ as *mut _;
    let cell = cell::Cell::new(ptr);
    unsafe {
        *cell.get() = val;
    }
}

fn dyn_cast<T: Copy>(var: &dyn any::Any) -> T {
    let ptr = var as *const _ as *const _;
    unsafe { *ptr }
}

const fn target_endian() -> &'static str {
    if u16::from_ne_bytes([1, 0]) == 1 {
        "Little Endian"
    } else {
        "Big Endian"
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;
    #[test]
    fn dyn_any() {
        tdbg!(target_endian());

        let x = [&mut 7 as &dyn any::Any, &4.3];
        let y = 123;
        dbg!(y, dyn_cast::<char>(&y));
        dyn_set(&y, 456);
        dbg!(y, dyn_cast::<char>(&y));

        dyn_set(x[0], "rust");
        dbg!(dyn_cast::<&str>(x[0]));
        dyn_set(x[0], -123);
        dbg!(dyn_cast::<u8>(x[0]));

        dbg!(dyn_cast::<f32>(x[1]));
        dbg!(dyn_cast::<&str>(x[1]));
        dbg!(dyn_cast::<f64>(x[1]));

        let mut z = 111;
        dbg!(&mem::replace(&mut z, 128));
        dbg!(&z);
    }
}
