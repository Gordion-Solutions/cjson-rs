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
            // Reserve header space so `install.sh`'s install_name_tool can
            // rewrite the install name to an absolute path of any length.
            println!("cargo:rustc-cdylib-link-arg=-Wl,-headerpad_max_install_names");
        }
        Ok("linux") => {
            println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,libcjson.so.1");
            // Export control is left to rustc's own generated version script,
            // which exports only this crate's `#[no_mangle]` symbols
            // (cJSON_* / cJSONUtils_*) and hides the Rust runtime. Passing our
            // own named version script here as well makes `ld` reject the mix
            // ("anonymous version tag cannot be combined with other version
            // tags"); the CI symbol-export check guards that nothing leaks.
        }
        _ => {}
    }
    println!("cargo:rerun-if-changed=exported_symbols.txt");
}
