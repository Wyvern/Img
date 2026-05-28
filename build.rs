use std::*;

fn main() {
    let input = "src/web.json";
    let output = "web.cbor";
    println!("cargo::rerun-if-changed={input}");

    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env.starts_with("musl") {
        println!("cargo::rustc-link-lib=m");
    }

    use fs::*;
    use io::*;

    let json_file = File::open(input).unwrap();
    let reader = BufReader::new(json_file);
    let value: serde_json::Value = serde_json::from_reader(reader).unwrap();

    let cbor_file = File::create(output).unwrap();
    let writer = BufWriter::new(cbor_file);
    cbor4ii::serde::to_writer(writer, &value).unwrap();

    let family = std::env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default();

    let mut cmd = if family == "windows" {
        let mut c = process::Command::new("tar");
        c.args(["-czf", "web.tar.gz", output]);
        c
    } else if family == "unix" {
        let mut c = process::Command::new("gzip");
        c.args(["-kf", output]);
        c
    } else {
        return;
    };
    assert!(cmd.status().unwrap().success());
}

#[test]
fn build() {
    main();
}

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
