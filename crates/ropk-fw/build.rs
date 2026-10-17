use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let debug = env::var_os("CARGO_FEATURE_DEBUG").is_some();
    let uf2 = env::var_os("CARGO_FEATURE_UF2").is_some();
    if debug == uf2 {
        panic!("if you got here you fucked up, use the commands");
    }

    let layout = if uf2 { "memory_prod.x" } else { "memory_debug.x" };

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::copy(layout, out.join("memory.x")).unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory_debug.x");
    println!("cargo:rerun-if-changed=memory_prod.x");
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
}
