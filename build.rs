use std::*;

fn main() {
    let target = env::var("TARGET").expect("TARGET environment variable is not set");
    let os = target.split('-').nth(2).unwrap_or("unknown");
    let abi = target.split('-').nth(3).unwrap_or("unknown");
    // D!(target_os);
    match os {
        "freebsd" => {
            println!("cargo::rustc-link-arg=-Wl,--allow-multiple-definition");
            println!("cargo::rustc-link-arg=-Wl,--export-dynamic");
        }
        "solaris" | "illumos" => {
            println!("cargo::rustc-link-arg=-Wl,-z,noexecstack");
            println!("cargo::rustc-link-arg=-Wl,-z,nocopyreloc");
        }
        _ => (),
    }
    match abi {
        "ohos" => {
            // println!("cargo::rustc-link-arg=-Wl,--no-undefined");
            // println!("cargo::rustc-link-arg=-Wl,--as-needed");
        }
        _ => (),
    }

    println!("cargo::rerun-if-changed=build.rs");
    // println!("cargo::rustc-link-lib=lib");
    // println!("cargo::rerun-if-env-changed=file");
    // println!("cargo::rustc-flags=-l{}", "");
    // println!("cargo::rustc-check-cfg=cfg(x,y)");
    // println!("cargo::rustc-cfg=x");
}

#[test]
fn build() {
    main();
}

#[macro_export]
macro_rules! D {
    () => {
        #[cfg(debug_assertions)]{
            $crate::println!("cargo::warning=[{}:{}:{}]", $crate::file!(), $crate::line!(), $crate::column!())
        }
    };
    ($val:expr $(,)?) => {
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
    ($($val:expr),+ $(,)?) => {
        ($(D!($val)),+,)
    };
}
