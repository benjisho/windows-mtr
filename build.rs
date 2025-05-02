fn main() {
    // Set Windows subsystem to console for proper terminal handling
    #[cfg(windows)]
    {
        println!("cargo:rustc-link-arg=/SUBSYSTEM:CONSOLE");
    }
        
    // Windows cross-compilation support
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        // Ensure we can find windows libraries when cross-compiling
        println!("cargo:rustc-link-lib=iphlpapi");
        println!("cargo:rustc-link-lib=ws2_32");
        
        // When cross-compiling, explicitly set MSVC environment
        if cfg!(not(windows)) {
            println!("cargo:warning=Cross-compiling Windows binary from non-Windows environment");
            println!("cargo:rustc-link-search=native=xwin-dlls");
        }
    }
}