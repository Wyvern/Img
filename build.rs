use std::*;

fn main() {
    println!("cargo::rerun-if-changed=src/web.json");

    use fs::*;
    use io::*;

    let json_file = File::open("src/web.json").unwrap();
    let reader = BufReader::new(json_file);
    let value: serde_json::Value = serde_json::from_reader(reader).unwrap();

    let cbor_file = File::create("src/web.cbor").unwrap();
    let writer = BufWriter::new(cbor_file);
    cbor4ii::serde::to_writer(writer, &value).unwrap();

    #[cfg(unix)]
    {
        let output = process::Command::new("gzip")
            .args(["-kf", "src/web.cbor"])
            .output()
            .unwrap();
        assert!(output.status.success());
    }

    #[cfg(any(windows, target_abi = "llvm"))]
    {
        let output = process::Command::new("tar")
            .args(["-czf", "src/web.tar.gz", "src/web.cbor"])
            .output()
            .unwrap();
        assert!(output.status.success());
    }
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
