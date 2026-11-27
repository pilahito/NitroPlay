// Enlaza la libreria C de referencia ya compilada y genera los bindings desde las cabeceras reales.
// Variables de entorno:
//   NITROPLAY_REF_BUILD_DIR  carpeta del build de CMake (con lib/libchiaki.a y third-party/...)
//   NITROPLAY_REF_SRC_DIR    raiz del repositorio (por defecto ../.. respecto a este crate)
use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=NITROPLAY_REF_BUILD_DIR");
    println!("cargo:rerun-if-env-changed=NITROPLAY_REF_SRC_DIR");
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = env::var("NITROPLAY_REF_SRC_DIR").map(PathBuf::from).unwrap_or_else(|_| manifest.join("../.."));
    let include = src.join("lib/include");

    let bindings = bindgen::Builder::default()
        .header(include.join("chiaki/common.h").to_string_lossy())
        .header(include.join("chiaki/regist.h").to_string_lossy())
        .header(include.join("chiaki/session.h").to_string_lossy())
        .header(include.join("chiaki/discovery.h").to_string_lossy())
        .header(include.join("chiaki/controller.h").to_string_lossy())
        .clang_arg(format!("-I{}", include.display()))
        .allowlist_function("chiaki_.*")
        .allowlist_type("Chiaki.*")
        .allowlist_var("CHIAKI_.*")
        .derive_default(true)
        .layout_tests(false)
        .generate()
        .expect("bindgen no pudo leer las cabeceras de la libreria de referencia");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings.write_to_file(out.join("bindings.rs")).unwrap();

    if let Ok(dir) = env::var("NITROPLAY_REF_BUILD_DIR") {
        let d = PathBuf::from(dir);
        for sub in ["lib", "third-party", "third-party/nanopb", "third-party/curl/lib"] {
            println!("cargo:rustc-link-search=native={}", d.join(sub).display());
        }
        println!("cargo:rustc-link-lib=static=chiaki");
        println!("cargo:rustc-link-lib=static=protobuf-nanopb");
        println!("cargo:rustc-link-lib=static=jerasure");
        println!("cargo:rustc-link-lib=static=gf_complete");
        println!("cargo:rustc-link-lib=static=curl");
        for l in ["json-c", "opus", "ssl", "crypto", "miniupnpc", "event", "z", "idn2", "pthread"] {
            println!("cargo:rustc-link-lib={l}");
        }
    }
}
