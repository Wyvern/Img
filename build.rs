use std::*;

#[path = "src/util.rs"]
mod util;

fn main() {
    let input = "src/web.json";
    let output = "web.cbor";
    println!("cargo::rerun-if-changed={input}");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS");
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV");
    let target_family = std::env::var("CARGO_CFG_TARGET_FAMILY");

    if target_env.is_ok_and(|e| e.starts_with("musl")) {
        println!("cargo::rustc-link-lib=m");
    }
    if target_os.as_deref() == Ok("espidf") {
        #[cfg(target_os = "espidf")]
        embuild::espidf::sysenv::output();
    }

    use fs::*;
    use io::*;

    let json_file = File::open(input).unwrap();
    let reader = BufReader::new(json_file);
    let value: serde_json::Value = serde_json::from_reader(reader).unwrap();

    let cbor_file = File::create(output).unwrap();
    let writer = BufWriter::new(cbor_file);
    cbor4ii::serde::to_writer(writer, &value).unwrap();

    let mut cmd = match target_family.as_deref() {
        Ok("windows") => {
            let mut c = process::Command::new("tar");
            c.args(["-czf", "web.tar.gz", output]);
            c
        }
        Ok(t) if t.contains("unix") && target_os.as_deref() != Ok("espidf") => {
            let mut c = process::Command::new("gzip");
            c.args(["-kf", output]);
            c
        }
        _ => return,
    };
    assert!(cmd.status().unwrap().success());
}

#[test]
fn build() {
    main();
}
