use std::{os::raw::c_int, *};

macro_rules! Color {
            ($($i:ident = $l:literal),+) => {
                STATIC!(pub &str;$($i=concat!("\x1b[",$l,'m')),+);
            }
        }

macro_rules! STATIC {
            ($v:vis $t:ty; $($i:ident = $e:expr),+) => {
                $(#[allow(unused)] $v static $i: $t = $e;)+
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
    MARK = "\x1b]1337;SetMark\x07"
);

Color!(
    N = 0,
    B = 1,
    U = 4,
    _U = 24,
    R = 91,
    G = 92,
    Y = 93,
    HL = 103
);

mod macros {
    #[macro_export]
    macro_rules! cdbg {
        () => {
            #[cfg(debug_assertions)]{
                $crate::println!("cargo::warning=[{}:{}:{}]", $crate::file!(), $crate::line!(), $crate::column!())
            }
        };
        ($val:expr) => {
            #[cfg(debug_assertions)]{
                match $val {
                tmp => {
                        $crate::println!("cargo::warning=[{}:{}:{}] {} = {:#?}",
                            $crate::file!(), $crate::line!(), $crate::column!(), $crate::stringify!($val), &tmp);
                        tmp
                    }
                }
            }
        };
        ($val:expr;) => {
            #[cfg(debug_assertions)]{
                match $val {
                tmp => {
                        $crate::println!("cargo::error=[{}:{}:{}] {} = {:#?}",
                            $crate::file!(), $crate::line!(), $crate::column!(), $crate::stringify!($val), &tmp);
                        tmp
                    }
                }
            }
        };
        ($($val:expr),+) => {
            ($(cdbg!($val)),+)
        };
        ($($val:expr),+;) => {
            ($(cdbg!($val;)),+)
        };
    }

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
    macro_rules! current_fn {
        () => {{
            fn f() {}
            let name = any::type_name_of_val(&f);
            &name[..name.len() - 3]
        }};
    }

    #[macro_export]
    macro_rules! mv {
        ($var:ident = $val:expr) => {
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
            cfg_select! {
                any(test,debug_assertions)=>{dbg!(($($e),*))}
                _=>{#[allow(unused)]($($e),*)}
            }
        };
        ($($e:expr),*;) => {
            {
                cfg_select! {
                    any(test,debug_assertions)=>{
                        let _l = io::stdout().lock();
                        let r = dbg!(($($e),*));
                        pause();
                        r
                    }
                    _=>{#[allow(unused)]($($e),*)}
                }
            }
        }
    }

    macro_rules! _demo {
    ([$attr:meta ] $pub:vis & $lt:lifetime $pp:pat_param in $name:ident : $type:ty =$e2:expr_2021, | $l:literal | $e:expr, $s:stmt ; $pat:pat => $b:block | $p:path | $i:item | $t:tt) => {$pat $t};

    ($id:ident, $b:block, $stmt:stmt, $e:expr, $pat:pat, $t:ty, $lt:lifetime, $l:literal, $p:path, $m:meta, $tt:tt, $i:item, $v:vis, $e2:expr_2021, $pp:pat_param)=>{};

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

#[allow(unused)]
pub trait AsBytes: Sized {
    fn from_bytes(bytes: &[u8]) -> &Self {
        assert_eq!(bytes.len(), mem::size_of::<Self>(), "slice size mismatch.");
        unsafe { &*(bytes.as_ptr() as *const Self) }
    }
    fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts((self as *const Self).cast::<u8>(), mem::size_of::<Self>()) }
    }
    fn eql<Other>(&self, other: &Other) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}
impl<T: Sized> AsBytes for T {}

#[allow(unused)]
pub trait Dbg: fmt::Debug {
    fn dbgr(&self) -> &Self {
        crate::tdbg!(self)
    }
    fn dbg(self) -> Self
    where
        Self: Sized,
    {
        crate::tdbg!(self)
    }
    fn dbgr_pause(&self) -> &Self {
        crate::tdbg!(self;)
    }
    fn dbg_pause(self) -> Self
    where
        Self: Sized,
    {
        crate::tdbg!(self;)
    }
}
impl<T: fmt::Debug> Dbg for T {}

#[cfg(all(unix, not(target_os = "emscripten")))]
pub fn begin_raw_mode(fd: c_int, mut old: mem::MaybeUninit<libc::termios>) {
    unsafe {
        libc::tcgetattr(fd, old.as_mut_ptr());
        let old = old.assume_init();
        let mut new = old;
        new.c_lflag &= !(libc::ICANON | libc::ECHO);
        libc::tcsetattr(fd, libc::TCSANOW, &new);
    }
}

#[cfg(all(unix, not(target_os = "emscripten")))]
pub fn end_raw_mode(fd: c_int, old: mem::MaybeUninit<libc::termios>) {
    unsafe {
        libc::tcsetattr(fd, libc::TCSANOW, old.as_ptr());
    }
}
#[allow(unused)]
pub fn pause() {
    use io::*;
    let mut o = stdout();
    write!(o, "Press any key to continue, or [Q̲]uit: ").unwrap();
    o.flush().unwrap();
    cfg_select! {
        windows=>{
            unsafe extern "C" {
                fn _getch() -> i32;
            }
            let ch=unsafe { _getch() };
            match ch {
                b'q' | b'Q' | 0x1b => {
                    write!(o, "{CL}⏏!").unwrap();
                    o.flush().unwrap();
                    process::exit(0);
                }
                _ => {
                    write!(o, "{CL}").unwrap();
                    o.flush().unwrap();
                }
            }
            }
        all(unix,not(target_os = "emscripten"))=>{
            let fd = libc::STDIN_FILENO;
            let old = mem::MaybeUninit::<libc::termios>::uninit();
            begin_raw_mode(fd, old);
            let mut i = stdin();
            let mut key = [0u8; 1];
            i.read_exact(&mut key).unwrap();
            end_raw_mode(fd, old);
            match key[0] {
                b'q' | b'Q' | 0x1b => {
                    write!(o, "{CL}⏏!").unwrap();
                    o.flush().unwrap();
                    process::exit(0);
                }
                _ => {
                    write!(o, "{CL}").unwrap();
                    o.flush().unwrap();
                }
            }
        }
        _=>{
            let mut o = stdout().lock();
            write!(o, "Press any key to continue, or [Q̲]uit: ").unwrap();
            o.flush().unwrap();
            let mut s = String::default();
            stdin().lock().read_line(&mut s).unwrap();
            s.make_ascii_lowercase();
            if s.trim() == "q" {
                write!(o, "{UP}{CL}⏏!").unwrap();
                o.flush().unwrap();
                drop(o);
                process::exit(0);
            } else {
                write!(o, "{UP}{CL}").unwrap();
                o.flush().unwrap();
            }
        }
    };
}

#[cfg(test)]
mod test {
    use super::*;

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
