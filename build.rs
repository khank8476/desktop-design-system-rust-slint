fn main() {
    println!("cargo:rerun-if-changed=ui/");
    println!("cargo:rerun-if-changed=examples/crypto_tracker.slint");
    println!("cargo:rerun-if-changed=examples/pos_terminal.slint");

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
    if std::path::Path::new("examples/pos_terminal.slint").exists() {
        slint_build::compile_with_output_path(
            "examples/pos_terminal.slint",
            out_dir.join("pos_terminal.rs"),
            slint_build::CompilerConfiguration::new(),
        ).expect("Failed to compile examples/pos_terminal.slint");
    }
}

