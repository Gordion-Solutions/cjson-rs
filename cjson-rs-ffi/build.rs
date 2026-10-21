//! Link settings for the shared library: export only the cJSON_* symbols
//! (Rust runtime symbols stay hidden, so two Rust libraries in one process
//! don't collide), and give it cJSON's own names so programs built against
//! the real libcjson load it unchanged (SOVERSION 1: libcjson.so.1,
//! libcjson.1.dylib).

fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("macos") => {
            println!("cargo:rustc-cdylib-link-arg=-Wl,-install_name,@rpath/libcjson.1.dylib");
            println!("cargo:rustc-cdylib-link-arg=-Wl,-exported_symbols_list,{dir}/exported_symbols.txt");
        }
        Ok("linux") => {
            println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,libcjson.so.1");
            println!("cargo:rustc-cdylib-link-arg=-Wl,--version-script={dir}/cjson.map");
        }
        _ => {}
    }
    println!("cargo:rerun-if-changed=exported_symbols.txt");
    println!("cargo:rerun-if-changed=cjson.map");
}
