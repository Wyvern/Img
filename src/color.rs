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

    DoubleUnderline = 21,
    BoldOff = 22,
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

    ResetFG = 39,

    BlackBg = 40,
    RedBg = 41,
    GreenBg = 42,
    YellowBg = 43,
    BlueBg = 44,
    MagentaBg = 45,
    CyanBg = 46,
    WhiteBg = 47,

    ResetBG = 49,

    Frame = 51,
    Encircle = 52,
    Overline = 53,
    FrameEncircleOff = 54,
    OverlineOff = 55,

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
            0..=9 | 21..=25 | 27..=29 | 30..=37 | 39..=47 | 49 | 51..=55 | 90..=97 | 100..=107 => unsafe {
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

#[derive(argh::FromArgs, Debug)]
#[argh(
    description = "color example app",
    name = "color",
    example = "{command_name} rgb 12 34 56"
)]
struct Args {
    #[argh(subcommand)]
    cmd: Option<Command>,
}

#[derive(argh::FromArgs, Debug)]
#[argh(subcommand)]
enum Command {
    Fg(FgCmd),
    Bg(BgCmd),
    Rgb(RgbCmd),
}

#[derive(argh::FromArgs, Debug)]
#[argh(subcommand, name = "fg", description = "foreground color")]
struct FgCmd {
    #[argh(positional)]
    color: Option<String>,
}

#[derive(argh::FromArgs, Debug)]
#[argh(subcommand, name = "bg", description = "background color")]
struct BgCmd {
    #[argh(positional)]
    color: Option<String>,
}

#[derive(argh::FromArgs, Debug)]
#[argh(subcommand, name = "rgb", description = "r g b 24-bit full color mode")]
struct RgbCmd {
    #[argh(positional)]
    rgb: Vec<String>,
}

fn parse_u8(mut s: String) -> Result<u8, num::ParseIntError> {
    s.make_ascii_lowercase();
    s.parse::<u8>()
        .or_else(|_| u8::from_str_radix(s.trim_start_matches("0x"), 16))
}

fn main() {
    let args: Args = argh::from_env();
    match args.cmd {
        Some(Command::Fg(cmd)) => {
            if let Some(s) = cmd.color
                && let Ok(n) = parse_u8(s)
            {
                println!("{:?}", Color::FG(n));
            } else {
                (0u8..=255).for_each(|c| {
                    println!("{:?}", Color::FG(c));
                });
            }
        }

        Some(Command::Bg(cmd)) => {
            if let Some(s) = cmd.color
                && let Ok(n) = parse_u8(s)
            {
                println!("{:?}", Color::BG(n));
            } else {
                (0u8..=255).for_each(|c| {
                    println!("{:?}", Color::BG(c));
                });
            }
        }

        Some(Command::Rgb(cmd)) => {
            if cmd.rgb.len() != 3 {
                eprintln!("rgb requires exactly 3 values");
                process::exit(1);
            }

            let mut rgb = cmd.rgb.into_iter().map(|x| parse_u8(x));

            if let [Ok(r), Ok(g), Ok(b)] = [
                rgb.next().unwrap(),
                rgb.next().unwrap(),
                rgb.next().unwrap(),
            ] {
                println!("{:?}", Color::RGBfg(r, g, b));
                println!("{:?}", Color::RGBbg(r, g, b));
            } else {
                eprintln!("r, g, b should be in range [0..255] or 0x00..0xff");
                process::exit(1);
            }
        }

        None => {
            color8();
        }
    }
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
