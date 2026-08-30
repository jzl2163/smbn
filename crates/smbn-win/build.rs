use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("app.manifest");
        println!("cargo:rustc-link-arg-bin=smbn=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-bin=smbn=/MANIFESTINPUT:{}", manifest.display());
    }
}
