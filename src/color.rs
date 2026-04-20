use std::*;

mod util;
static TEXT: &str = "The quick brown fox jumps over the lazy dog.";

#[repr(u8)]
pub enum Color {
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

    FG(u8),
    BG(u8),
    RGBfg(u8, u8, u8),
    RGBbg(u8, u8, u8),
}
impl Color {
    const fn code(&self) -> u8 {
        unsafe { *(self as *const Self).cast() }
    }
}
impl From<u8> for Color {
    fn from(value: u8) -> Self {
        match value {
            0..=9 | 22..=25 | 27..=29 | 30..=37 | 40..=47 | 90..=97 | 100..=107 => unsafe {
                mem::transmute(value as u32)
            },
            _ => panic!("Out of range of value for Color enum."),
        }
    }
}
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FG(c) => write!(f, "\x1b[38;5;{c}m"),
            Self::BG(c) => write!(f, "\x1b[48;5;{c}m"),
            Self::RGBfg(r, g, b) => write!(f, "\x1b[38;2;{r};{g};{b}m"),
            Self::RGBbg(r, g, b) => write!(f, "\x1b[48;2;{r};{g};{b}m"),
            _ => write!(f, "\x1b[{}m", self.code()),
        }
    }
}
impl fmt::Debug for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FG(c) => writeln!(
                f,
                "\"\\x1b[38;5;{c}m\": - \x1b[38;5;{c}m{TEXT}{}",
                Self::Reset
            ),
            Self::BG(c) => writeln!(
                f,
                "\"\\x1b[48;5;{c}m\": - \x1b[48;5;{c}m{TEXT}{}",
                Self::Reset
            ),
            Self::RGBfg(r, g, b) => {
                writeln!(
                    f,
                    "\"\\x1b[38;2;{r};{g};{b}m\": - \x1b[38;2;{r};{g};{b}m{TEXT}{}",
                    Self::Reset
                )
            }
            Self::RGBbg(r, g, b) => {
                writeln!(
                    f,
                    "\"\\x1b[48;2;{r};{g};{b}m\": - \x1b[48;2;{r};{g};{b}m{TEXT}{}",
                    Self::Reset
                )
            }
            _ => writeln!(
                f,
                "\"\\x1b[{0}m\": - \x1b[{0}m{TEXT}{1}",
                self.code(),
                Self::Reset
            ),
        }
    }
}

fn main() {
    use nanoargs::*;
    let parser = ArgBuilder::new()
        .name("color")
        .version("1.0.0")
        .description("show various colors in terminal.")
        .subcommand(
            "fg",
            "- [value] Foreground color",
            ArgBuilder::new()
                .positional(Pos::new("color").validate(range(0, 255)))
                .build()
                .unwrap(),
        )
        .subcommand(
            "bg",
            "- [value] Background color",
            ArgBuilder::new()
                .positional(Pos::new("color").validate(range(0, 255)))
                .build()
                .unwrap(),
        )
        .subcommand(
            "rgb",
            "- r g b 24-bit full color mode",
            ArgBuilder::new()
                .positional(Pos::new("code").multi())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    match parser.parse_env() {
        Ok(res) => match res.subcommand() {
            fb @ Some("fg" | "bg") => {
                let r = res.subcommand_result().unwrap();
                let pos = r.get_positionals();
                if pos.is_empty() {
                    (0u8..=255).for_each(|c| {
                        println!(
                            "{:?}",
                            if let Some("fg") = fb {
                                Color::FG(c)
                            } else {
                                Color::BG(c)
                            }
                        )
                    });
                } else {
                    println!(
                        "{:?}",
                        if let Some("fg") = fb {
                            Color::FG(pos[0].parse::<u8>().unwrap())
                        } else {
                            Color::BG(pos[0].parse::<u8>().unwrap())
                        }
                    );
                }
            }
            Some("rgb") => {
                let r = res.subcommand_result().unwrap();
                let rgb = r.get_positionals();
                if rgb.len() != 3 {
                    print!("{}", parser.help_text());
                    process::exit(0);
                }
                let mut rgb = rgb.iter().map(|c| {
                    c.parse::<u8>()
                        .or_else(|_| u8::from_str_radix(c.trim_start_matches("0x"), 16))
                });

                if let [Ok(r), Ok(g), Ok(b)] = [
                    rgb.next().unwrap(),
                    rgb.next().unwrap(),
                    rgb.next().unwrap(),
                ] {
                    println!("{:?}", Color::RGBfg(r, g, b));
                    println!("{:?}", Color::RGBbg(r, g, b));
                } else {
                    println!("r, g, b should be in range of [0..255] / 0x[00..ff]");
                }
            }
            _ => {}
        },
        Err(ParseError::HelpRequested(text) | ParseError::VersionRequested(text)) => {
            print!("{text}");
        }
        Err(ParseError::NoSubcommand(_)) => color8(),
        Err(ParseError::UnknownSubcommand(x)) => {
            let c = x
                .parse::<u8>()
                .or_else(|_| u8::from_str_radix(x.trim_start_matches("0x"), 16));
            if let Ok(v) = c {
                println!("{:?}", Color::FG(v));
                println!("{:?}", Color::BG(v));
            } else {
                print!("{}", parser.help_text());
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
        }
    };
}

#[cfg(test)]
mod color {
    use super::*;

    #[test]
    fn color() {
        println!(
            "{:?}{:?}{:?}{:?}",
            Color::Magenta,
            Color::FG(Color::BrightGreen.code()),
            Color::BG(Color::BrightBlue.code()),
            Color::RGBbg(100, 200, 220)
        );
    }
    #[test]
    fn run() {
        main();
    }
}

fn color8() {
    fn section(s: &str) {
        println!("\n{}{}{s}:{}", Color::Bold, Color::Underline, Color::Reset)
    }
    (0u8..=9)
        .chain(22..=25)
        .chain(27..=29)
        .chain(30..=37)
        .chain(40..=47)
        .chain(90..=97)
        .chain(100..=107)
        .for_each(|c| {
            match c {
                0 => section("Basic Style"),
                22 => section("Advanced Style"),
                30 => section("8-color regular foreground"),
                40 => section("8-color regular background"),
                90 => section("8-color bright foreground"),
                100 => section("8-color bright background"),
                _ => (),
            };
            println!("{:?}", Color::from(c));
        });
}
