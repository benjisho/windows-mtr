fn main() {
    // Set Windows subsystem to console for proper terminal handling
    #[cfg(windows)]
    {
        println!("cargo:rustc-link-arg=/SUBSYSTEM:CONSOLE");
        
        // Windows cross-compilation support when building on non-windows
        if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
            // Ensure we can find windows libraries when cross-compiling
            println!("cargo:rustc-link-lib=iphlpapi");
            println!("cargo:rustc-link-lib=ws2_32");
        }
    }
}