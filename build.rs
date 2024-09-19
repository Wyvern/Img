use std::*;

fn main() {
    let out = env::var("OUT_DIR");
    let target = env::var("TARGET");

    if let Ok(o) = out {
        if let Ok(t) = target {
            D!(&t, &o);

            if t == "arm64ec-pc-windows-msvc" {
                println!("cargo::rustc-flags=-Clinker=link.exe");
                println!("cargo::rustc-link-arg=");
                println!("cargo::rustc-link-arg-bins=");
                println!("cargo::rustc-link-search={}", o);
                // println!("cargo::rustc-link-lib=");
            }
        }
    }

    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=TARGET");
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
