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
macro_rules! Color {
            ($($i:ident = $l:literal),+) => {
                STATIC!(pub &str;$($i=concat!("\x1b[",$l,'m')),+);
            }
        }

macro_rules! STATIC {
            ($v:vis $t:ty; $($i:ident = $e:expr),+) => {
                $($v static $i: $t = $e;)+
            }
        }

// macro_rules! CONST {
//             ($v:vis $t:ty; $($i:ident = $e:expr),+) => {
//                 $($v const $i: $t = $e;)+
//             }
//         }

STATIC!(pub &str;
    UP = "\x1b[1A",
    CL = "\r\x1b[2K", //Clear current line + move to start
    MARK = "\x1b]1337;SetMark\x07",
    TEXT = "The quick brown fox jumps over the lazy dog"
);

Color!(
    N = 0,
    B = 1,
    _B = 22,
    I = 3,
    _I = 23,
    U = 4,
    _U = 24,
    UU = 21,
    R = 91,
    G = 92,
    Y = 93,
    BLUE = 94,
    HL = 103,
    BG = 100,
    FG = 97
);

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
        ($l:literal $(,$e:expr)*) => {
            println!("{B}{}{N}", format_args!($l $(,format_args!("`{R}{}{N}{B}`",$e))*))
        }
    }

    #[macro_export]
    macro_rules! mutv {
        ($var:ident, $val:expr) => {
            unsafe {
                *(&raw const $var).cast_mut() = $val;
            }
        };
    }

    #[macro_export]
    macro_rules! p {
        ($l:literal $(,$e:expr)*) => {
            print!("{B}{}{N}", format_args!($l $(,format_args!("`{R}{}{N}{B}`",$e))*))
        }
    }

    #[macro_export]
    macro_rules! tdbg {
        ($($e:expr),*) => {
            if cfg!(test) || cfg!(debug_assertions) {
                dbg!(($($e),*))
            } else {($($e),*)}
        };
        ($($e:expr),*;) => {
            if cfg!(test) || cfg!(debug_assertions) {
                let _l = io::stdout().lock();
                let r = dbg!(($($e),*));
                pause();
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

pub trait AsBytes: Sized {
    fn from_bytes(bytes: &[u8]) -> &Self {
        unsafe { &*(bytes.as_ptr() as *const Self) }
    }
    fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts((self as *const Self).cast::<u8>(), mem::size_of::<Self>()) }
    }
    fn eql<Other>(&self, other: &Other) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}
impl<T> AsBytes for T {}

pub trait Dbg: fmt::Debug {
    fn dbg(&self) {
        crate::tdbg!(self);
    }
    fn dbg_pause(&self) {
        crate::tdbg!(self;);
    }
}
impl<T: fmt::Debug> Dbg for T {}

pub fn pause() {
    use io::*;
    #[cfg(target_family = "unix")]
    {
        use termion::input::TermRead;
        use termion::raw::IntoRawMode;

        let mut o = stdout().into_raw_mode().unwrap();
        write!(o, "Press any key to continue, or [Q̲]uit: ").unwrap();
        o.flush().unwrap();

        let i = stdin();
        if let Some(Ok(termion::event::Key::Char('q') | termion::event::Key::Char('Q'))) =
            i.keys().next()
        {
            write!(o, "{CL}Quit!").unwrap();
            o.flush().unwrap();
            drop(o);
            process::exit(0);
        } else {
            write!(o, "{CL}").unwrap();
            o.flush().unwrap();
        }
    }
    #[cfg(not(target_family = "unix"))]
    {
        let mut o = stdout().lock();
        write!(o, "Press any key to continue, or [Q̲]uit: ").unwrap();
        o.flush().unwrap();
        let mut s = String::default();
        stdin().lock().read_line(&mut s).unwrap();
        s.make_ascii_lowercase();
        if s.trim() == "q" {
            write!(o, "{UP}{CL}Quit!").unwrap();
            o.flush().unwrap();
            drop(o);
            process::exit(0);
        } else {
            write!(o, "{UP}{CL}").unwrap();
            o.flush().unwrap();
        }
    }
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

    #[test]
    fn dyn_any() {
        crate::tdbg!(target_endian());

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
