use std::*;

fn main() {
    let target = env::var("TARGET").expect("TARGET environment variable is not set");
    let parts = [2, 3];
    let [_os, _abi] = array::from_fn(|n| target.split('-').nth(parts[n]).unwrap_or("unknown"));

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
