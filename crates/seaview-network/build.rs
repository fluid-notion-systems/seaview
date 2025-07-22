//! Build script for generating C headers with cbindgen

use std::env;
use std::path::PathBuf;

fn main() {
    // Only generate bindings when the ffi feature is enabled
    if env::var("CARGO_FEATURE_FFI").is_ok() {
        let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let out_dir = PathBuf::from(&crate_dir).join("include");

        // Create include directory if it doesn't exist
        std::fs::create_dir_all(&out_dir).unwrap();

        // Generate the bindings
        let output_file = out_dir.join("seaview_network.h");

        match cbindgen::Builder::new()
            .with_crate(&crate_dir)
            .with_language(cbindgen::Language::C)
            .with_header(
                r#"/*
 * seaview-network C API
 *
 * This header provides C bindings for the seaview-network Rust library,
 * enabling real-time mesh streaming from C/C++ applications.
 */

#ifndef SEAVIEW_NETWORK_H
#define SEAVIEW_NETWORK_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include <stddef.h>
"#,
            )
            .with_trailer(
                r#"
#ifdef __cplusplus
}
#endif

#endif /* SEAVIEW_NETWORK_H */
"#,
            )
            .with_cpp_compat(true)
            .with_documentation(true)
            .with_parse_deps(false)
            .with_parse_expand(&["ffi"])
            .generate()
        {
            Ok(bindings) => {
                bindings.write_to_file(&output_file);
                println!("cargo:rerun-if-changed=src/ffi.rs");
                println!("cargo:rerun-if-changed=build.rs");
                println!("Generated C header at: {}", output_file.display());
            }
            Err(e) => {
                eprintln!("Warning: Unable to generate C bindings: {}", e);
                eprintln!("This is expected if building without the ffi feature");
            }
        }
    }
}
