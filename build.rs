fn main() {
    println!("cargo:rerun-if-changed=ui/");
    println!("cargo:rerun-if-changed=examples/crypto_tracker.slint");

    slint_build::compile("ui/app.slint").expect("Failed to compile Slint UI files");

    // Compile standalone isolated examples without bloating main crate
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    if std::path::Path::new("examples/crypto_tracker.slint").exists() {
        let _ = slint_build::compile_with_output_path(
            "examples/crypto_tracker.slint",
            out_dir.join("crypto_tracker.rs"),
            slint_build::CompilerConfiguration::new(),
        );
    }
}

