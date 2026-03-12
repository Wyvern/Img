use {std::*, util::*};
mod util;
static TEXT: &str = "The quick brown fox jumps over the lazy dog.";

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Basic {
    Reset = 0,
    Bold = 1,
    Faint = 2,
    Italic = 3,
    Underline = 4,
    SlowBlink = 5,
    RapidBlink = 6,
    ReverseVideo = 7,
    Conceal = 8,
    Strikethrough = 9,

    NormalIntensity = 22,
    ItalicOff = 23,
    UnderlineOff = 24,
    BlinkOff = 25,
    ReverseOff = 27,
    Reveal = 28,
    StrikethroughOff = 29,

    Black = 30,
    Red = 31,
    Green = 32,
    Yellow = 33,
    Blue = 34,
    Magenta = 35,
    Cyan = 36,
    White = 37,

    BlackBg = 40,
    RedBg = 41,
    GreenBg = 42,
    YellowBg = 43,
    BlueBg = 44,
    MagentaBg = 45,
    CyanBg = 46,
    WhiteBg = 47,

    BrightBlack = 90,
    BrightRed = 91,
    BrightGreen = 92,
    BrightYellow = 93,
    BrightBlue = 94,
    BrightMagenta = 95,
    BrightCyan = 96,
    BrightWhite = 97,

    BrightBlackBg = 100,
    BrightRedBg = 101,
    BrightGreenBg = 102,
    BrightYellowBg = 103,
    BrightBlueBg = 104,
    BrightMagentaBg = 105,
    BrightCyanBg = 106,
    BrightWhiteBg = 107,
}
impl Basic {
    #[allow(unused)]
    fn code(&self) -> u8 {
        *self as u8
    }

    fn from(c: u8) -> Self {
        unsafe { mem::transmute(c) }
    }
}
impl fmt::Display for Basic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\x1b[{}m", self.code())
    }
}
impl fmt::Debug for Basic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\"\\x1b[{0}m\": - \x1b[{0}m{TEXT}{1}\n",
            self.code(),
            Self::Reset
        )
    }
}

pub enum Color {
    Basic(Basic),
    FG(u8),
    BG(u8),
    RGBfg(u8, u8, u8),
    RGBbg(u8, u8, u8),
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Basic(c) => write!(f, "{}", c),
            Self::FG(c) => write!(f, "\x1b[38;5;{c}m"),
            Self::BG(c) => write!(f, "\x1b[48;5;{c}m"),
            Self::RGBfg(r, g, b) => write!(f, "\x1b[38;2;{r};{g};{b}m"),
            Self::RGBbg(r, g, b) => write!(f, "\x1b[48;2;{r};{g};{b}m"),
        }
    }
}

impl fmt::Debug for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Basic(c) => write!(f, "{:?}", c),
            Self::FG(c) => write!(
                f,
                "\"\\x1b[38;5;{c}m\": - \x1b[38;5;{c}m{TEXT}{}\n",
                Basic::Reset
            ),
            Self::BG(c) => write!(
                f,
                "\"\\x1b[48;5;{c}m\": - \x1b[48;5;{c}m{TEXT}{}\n",
                Basic::Reset
            ),
            Self::RGBfg(r, g, b) => {
                write!(
                    f,
                    "\"\\x1b[38;2;{r};{g};{b}m\": - \x1b[38;2;{r};{g};{b}m{TEXT}{}\n",
                    Basic::Reset
                )
            }
            Self::RGBbg(r, g, b) => {
                write!(
                    f,
                    "\"\\x1b[48;2;{r};{g};{b}m\": - \x1b[48;2;{r};{g};{b}m{TEXT}{}\n",
                    Basic::Reset
                )
            }
        }
    }
}

fn main() {
    if env::args().len() > if cfg!(test) { 5 + 3 } else { 5 } {
        exit()
    }

    let args: [_; 4] = array::from_fn(|i| {
        if cfg!(test) {
            env::args().skip(3).nth(i + 1)
        } else {
            env::args().nth(i + 1)
        }
    });

    analyze_args(array::from_fn(|i| args[i].as_deref()));
}

fn analyze_args(args: [Option<&str>; 4]) {
    match args {
        [None, None, None, None] => color8(),
        [Some(v), None, None, None] if v.parse::<u8>() == Ok(8) => color8(),
        [Some(v), None, None, None] if v.parse::<u16>() == Ok(256) => {
            (0u8..=255).for_each(|c| println!("{:?}", Color::FG(c)));
            (0u8..=255).for_each(|c| println!("{:?}", Color::BG(c)));
        }
        [Some(v1), Some(v2), None, None] => match (
            v1.parse::<u16>().as_ref(),
            v2.parse::<u8>()
                .or_else(|_| u8::from_str_radix(v2.strip_prefix("0x").unwrap_or(v2), 16)),
        ) {
            (Ok(256), Ok(c)) => {
                println!("{:?}", Color::FG(c));
                println!("{:?}", Color::BG(c));
            }
            (Ok(256), _) => match v2 {
                "fg" => (0u8..=255).for_each(|c| println!("{:?}", Color::FG(c))),
                "bg" => (0u8..=255).for_each(|c| println!("{:?}", Color::BG(c))),
                _ => exit(),
            },
            _ => {
                exit();
            }
        },
        [Some("rgb") | Some("RGB"), Some(r), Some(g), Some(b)] => {
            match [r, g, b].map(|v| {
                v.parse::<u8>()
                    .or_else(|_| u8::from_str_radix(v.trim_start_matches("0x"), 16))
            }) {
                [Ok(r), Ok(g), Ok(b)] => {
                    println!("{:?}", Color::RGBfg(r, g, b));
                    println!("{:?}", Color::RGBbg(r, g, b));
                }
                _ => {
                    exit();
                }
            }
        }
        _ => {
            exit();
        }
    }
}

fn exit() {
    println!(
        "Usage: Color {NL}`8|256` {NL}`256 {B}<color>{N} [0-255]` {NL}`256 {{{B}{FG}fg,{BG}bg{N}}}` {NL}`RGB {B}{R}r {G}g {BLUE}b{N} [0-255]{{3}}` \n options.",
        NL = "\n\t",
    );
    process::exit(0);
}

#[cfg(test)]
mod color {
    use super::*;

    #[test]
    fn color() {
        color8();
        println!(
            "{:?}{:?}{:?}{:?}",
            Basic::Magenta,
            Color::FG(Basic::BrightGreen.code()),
            Color::BG(Basic::BrightBlue.code()),
            Color::RGBbg(100, 200, 220)
        );
    }
    #[test]
    fn run() {
        main();
    }
}

fn color8() {
    (0u8..=9)
        .chain(22..=25)
        .chain(27..=29)
        .chain(30..=37)
        .chain(40..=47)
        .chain(90..=97)
        .chain(100..=107)
        .for_each(|c| {
            _ = match c {
                0 => println!("\n{B}{U}Basic Style:{N}"),
                30 => println!("\n{B}{U}8-color regular foreground:{N}"),
                40 => println!("\n{B}{U}8-color regular background:{N}"),
                90 => println!("\n{B}{U}8-color bright foreground:{N}"),
                100 => println!("\n{B}{U}8-color bright background:{N}"),
                _ => (),
            };
            println!("{:?}", Basic::from(c));
        });
}
