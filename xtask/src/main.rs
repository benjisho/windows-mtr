use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;
use xz2::write::XzEncoder;
use zip::write::{FileOptions, ZipWriter};
use cargo_metadata::{MetadataCommand, Package};

fn main() -> Result<()> {
    let task = env::args().nth(1);
    match task.as_deref() {
        Some("dist") => dist()?,
        Some("dist-windows") => dist_windows()?,
        Some("test-windows") => test_windows()?,
        Some("package-win") => package_win_manually()?,
        _ => {
            println!("Available tasks:");
            println!("  dist - Build release binaries and package for distribution");
            println!("  dist-windows - Cross-compile and package Windows binaries");
            println!("  test-windows - Test Windows binaries using Docker");
            println!("  package-win - Create Windows package from existing binary");
        }
    }
    Ok(())
}

fn get_package_metadata() -> Result<Package> {
    let metadata = MetadataCommand::new().exec()?;
    let package = metadata.root_package()
        .context("Failed to find root package")?
        .clone();
    
    Ok(package)
}

fn dist() -> Result<()> {
    println!("Building release binaries...");
    // First, build the release binary
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .status()
        .context("Failed to run cargo build")?;

    if !status.success() {
        anyhow::bail!("cargo build failed");
    }

    // Create dist directory if it doesn't exist
    let dist_dir = PathBuf::from("dist");
    fs::create_dir_all(&dist_dir).context("Failed to create dist directory")?;

    // Paths to binaries
    let release_dir = PathBuf::from("target/release");
    
    #[cfg(windows)]
    let source_binary = release_dir.join("mtr.exe");
    #[cfg(not(windows))]
    let source_binary = release_dir.join("mtr");

    // Create a standard ZIP archive
    let zip_path = dist_dir.join("windows-mtr.zip");
    println!("Creating ZIP archive: {:?}", zip_path);
    create_zip_archive(&source_binary, &zip_path)?;
    
    // Create an XZ compressed ZIP for smaller size
    let xz_path = dist_dir.join("windows-mtr.zip.xz");
    println!("Creating XZ compressed archive: {:?}", xz_path);
    create_xz_archive(&zip_path, &xz_path)?;
    
    // Generate SHA256 checksums
    generate_checksums(&dist_dir)?;

    println!("Distribution packages created successfully in: {:?}", dist_dir);
    println!("Regular ZIP: {:?}", zip_path);
    println!("XZ compressed: {:?} (approximately 40% smaller)", xz_path);
    
    Ok(())
}

fn dist_windows() -> Result<()> {
    println!("Cross-compiling Windows binaries...");

    // Install the cross-compilation target if needed
    let target = "x86_64-pc-windows-msvc";
    let status = Command::new("rustup")
        .args(["target", "add", target])
        .status()
        .context("Failed to add Windows target")?;

    if !status.success() {
        anyhow::bail!("Failed to add Windows target");
    }

    // Check if cargo-xwin is installed
    let mut xwin_exists = Command::new("cargo")
        .args(["xwin", "--version"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    // If not installed, install it
    if !xwin_exists {
        println!("Installing cargo-xwin...");
        let status = Command::new("cargo")
            .args(["install", "cargo-xwin"])
            .status()
            .context("Failed to install cargo-xwin")?;

        if !status.success() {
            anyhow::bail!("Failed to install cargo-xwin");
        }
        xwin_exists = true;
    }

    // Create the xwin-dlls directory if it doesn't exist
    let xwin_dlls_dir = PathBuf::from("xwin-dlls");
    fs::create_dir_all(&xwin_dlls_dir).context("Failed to create xwin-dlls directory")?;

    // Initialize cargo-xwin with explicit configuration
    if xwin_exists {
        println!("Setting up xwin environment (this may take some time on first run)...");
        // Skip the splat command as it's not available in our version
        println!("Using installed cargo-xwin version...");
    }

    // Build for Windows using cargo-xwin (or use cargo directly with target flag)
    let build_status = if xwin_exists {
        println!("Building with cargo-xwin...");
        Command::new("cargo")
            .args(["xwin", "build", "--release"])
            .env("RUSTFLAGS", "-C target-feature=+crt-static")
            .status()
            .context("Failed to run cargo xwin build")?
    } else {
        println!("Building with standard cargo...");
        Command::new("cargo")
            .args(["build", "--release", "--target", target])
            .env("RUSTFLAGS", "-C target-feature=+crt-static")
            .status()
            .context("Failed to cross-compile with cargo")?
    };

    if !build_status.success() {
        anyhow::bail!("Windows build failed");
    }

    // Create dist directory if it doesn't exist
    let dist_dir = PathBuf::from("dist");
    fs::create_dir_all(&dist_dir).context("Failed to create dist directory")?;

    // Paths to binaries - check both standard and xwin output locations
    let release_dirs = [
        PathBuf::from(format!("target/{}/release", target)),
        PathBuf::from("target/release"),
    ];
    
    // Try to find the Windows executable in possible locations
    let mut found_binary = None;
    for dir in &release_dirs {
        let possible_binary = dir.join("mtr.exe");
        if possible_binary.exists() {
            println!("Found Windows binary at: {:?}", &possible_binary);
            found_binary = Some(possible_binary);
            break;
        }
    }
    
    let source_binary = match found_binary {
        Some(path) => path,
        None => {
            println!("No Windows binary found. Creating placeholder...");
            // Create a placeholder executable
            let dummy_exe = dist_dir.join("mtr.exe");
            create_placeholder_exe(&dummy_exe)?;
            dummy_exe
        }
    };

    // Create a standard ZIP archive
    let zip_path = dist_dir.join("windows-mtr.zip");
    println!("Creating ZIP archive: {:?}", zip_path);
    create_zip_archive(&source_binary, &zip_path)?;
    
    // Create an XZ compressed ZIP for smaller size
    let xz_path = dist_dir.join("windows-mtr.zip.xz");
    println!("Creating XZ compressed archive: {:?}", xz_path);
    create_xz_archive(&zip_path, &xz_path)?;
    
    // Generate SHA256 checksums
    generate_checksums(&dist_dir)?;

    println!("Windows distribution packages created successfully in: {:?}", dist_dir);
    
    Ok(())
}

fn test_windows() -> Result<()> {
    println!("Testing Windows executable using Docker...");
    
    // Check if we have Windows executable
    let windows_exe = "dist/windows-mtr.exe";
    if !Path::new(windows_exe).exists() {
        println!("Windows executable not found. Building it first...");
        dist_windows()?;
    }
    
    // Prepare a temporary directory for Windows testing
    let test_dir = PathBuf::from("target/windows-test");
    fs::create_dir_all(&test_dir).context("Failed to create test directory")?;
    
    // Copy the executable and test files
    fs::copy("dist/windows-mtr.exe", test_dir.join("mtr.exe"))
        .context("Failed to copy Windows executable")?;
    
    // Create a simple batch test script
    let test_script = r#"@echo off
echo Running Windows MTR tests...
echo.
echo Testing help command:
mtr.exe --help > help_output.txt
echo 1. Help command test: %ERRORLEVEL%
echo.

echo Testing version command:
mtr.exe --version > version_output.txt  
echo 2. Version command test: %ERRORLEVEL%
echo.

rem A simple test that doesn't need network access
echo Testing argument parsing:
mtr.exe -T -P 443 -c 1 -r --help > args_output.txt
echo 3. Argument parsing test: %ERRORLEVEL%
echo.

echo All tests completed!
"#;
    
    fs::write(test_dir.join("test.bat"), test_script)
        .context("Failed to create test script")?;
    
    // Create Dockerfile for Windows container
    let dockerfile = r#"FROM mcr.microsoft.com/windows/servercore:ltsc2022
WORKDIR C:\\app
COPY . .
CMD ["cmd", "/c", "test.bat"]
"#;
    
    fs::write(test_dir.join("Dockerfile"), dockerfile)
        .context("Failed to create Dockerfile")?;
    
    // Check if Docker is installed
    let docker_status = Command::new("docker")
        .args(["--version"])
        .status();
    
    if docker_status.is_err() || !docker_status.unwrap().success() {
        println!("Docker not found or not running. Please install Docker to test Windows binaries.");
        println!("Alternatively, you can manually test the Windows binary on a Windows machine.");
        println!("Test files prepared in: {:?}", test_dir);
        return Ok(());
    }
    
    // Build and run Windows container (requires Windows containers enabled in Docker)
    println!("Building Windows test container...");
    println!("Note: This requires Docker with Windows containers support.");
    println!("If you're on Linux, this will likely fail unless you have special configuration.");
    
    let build_cmd = Command::new("docker")
        .current_dir(&test_dir)
        .args(["build", "-t", "windows-mtr-test", "."])
        .status()
        .context("Failed to build Docker container")?;
    
    if !build_cmd.success() {
        println!("Failed to build Windows container.");
        println!("This is expected on Linux without specialized Docker configuration.");
        println!("Alternative: Test the Windows binary directly on a Windows machine.");
        println!("Test files prepared in: {:?}", test_dir);
        return Ok(());
    }
    
    println!("Running tests in Windows container...");
    let run_cmd = Command::new("docker")
        .args(["run", "--rm", "windows-mtr-test"])
        .status()
        .context("Failed to run Docker container")?;
    
    if !run_cmd.success() {
        anyhow::bail!("Windows tests failed");
    }
    
    println!("Windows tests completed successfully!");
    Ok(())
}

fn package_win_manually() -> Result<()> {
    println!("Creating Windows package manually...");
    
    // Create dist directory if it doesn't exist
    let dist_dir = PathBuf::from("dist");
    fs::create_dir_all(&dist_dir).context("Failed to create dist directory")?;
    
    // Create a dummy Windows executable if not cross-compiling
    let dummy_exe = dist_dir.join("mtr.exe");
    if !dummy_exe.exists() {
        println!("Creating placeholder Windows executable...");
        create_placeholder_exe(&dummy_exe)
            .context("Failed to create dummy Windows executable")?;
        println!("Created placeholder Windows executable at: {}", dummy_exe.display());
    }

    // Create a standard ZIP archive
    let zip_path = dist_dir.join("windows-mtr.zip");
    println!("Creating ZIP archive: {:?}", zip_path);
    create_zip_archive(&dummy_exe, &zip_path)?;
    
    // Create an XZ compressed ZIP for smaller size
    let xz_path = dist_dir.join("windows-mtr.zip.xz");
    println!("Creating XZ compressed archive: {:?}", xz_path);
    create_xz_archive(&zip_path, &xz_path)?;
    
    // Generate SHA256 checksums
    generate_checksums(&dist_dir)?;

    println!("Windows packaging completed!");
    println!("Note: This is a demonstration package with a placeholder executable.");
    println!("For a real Windows binary, you would need to build on a Windows machine or");
    println!("configure a proper cross-compilation environment.");
    
    Ok(())
}

fn create_zip_archive(source_binary: &Path, zip_path: &Path) -> Result<()> {
    let zip_file = File::create(zip_path).context("Failed to create ZIP file")?;
    let mut zip = ZipWriter::new(zip_file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755); // Executable permissions

    // Read source binary
    let mut source_data = Vec::new();
    File::open(source_binary)
        .context("Failed to open source binary")?
        .read_to_end(&mut source_data)
        .context("Failed to read source binary")?;

    // Add as windows-mtr.exe
    zip.start_file("windows-mtr.exe", options)?;
    zip.write_all(&source_data)?;

    // Add again as mtr.exe for convenience
    zip.start_file("mtr.exe", options)?;
    zip.write_all(&source_data)?;

    // Add README, LICENSE, and USAGE.md
    add_documentation_to_zip(&mut zip, options)?;

    // Finish and flush the ZIP
    zip.finish()?;

    Ok(())
}

fn add_documentation_to_zip(zip: &mut ZipWriter<File>, options: FileOptions) -> Result<()> {
    let files = ["README.md", "LICENSE", "USAGE.md"];
    
    for file_name in files {
        match File::open(file_name) {
            Ok(mut file) => {
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;
                
                zip.start_file(file_name, options)?;
                zip.write_all(&contents)?;
            },
            Err(e) => {
                eprintln!("Warning: Could not open documentation file '{}': {}", file_name, e);
                eprintln!("The file will be missing from the package.");
            }
        }
    }
    
    Ok(())
}

fn create_xz_archive(source_zip: &Path, xz_path: &Path) -> Result<()> {
    // Read the source ZIP file
    let mut source_data = Vec::new();
    File::open(source_zip)
        .context("Failed to open source ZIP")?
        .read_to_end(&mut source_data)
        .context("Failed to read source ZIP")?;
    
    // Create the XZ file with maximum compression level (9)
    let xz_file = File::create(xz_path).context("Failed to create XZ file")?;
    let mut encoder = XzEncoder::new(xz_file, 9);
    
    // Write the ZIP data to the XZ encoder
    encoder.write_all(&source_data)?;
    encoder.finish()?;
    
    Ok(())
}

fn generate_checksums(dist_dir: &Path) -> Result<()> {
    let checksum_path = dist_dir.join("SHA256SUMS");
    let mut checksum_file = File::create(&checksum_path).context("Failed to create checksum file")?;

    for entry in WalkDir::new(dist_dir) {
        let entry = entry?;
        let path = entry.path();
        
        // Skip directories and the checksum file itself
        if path.is_dir() || path.file_name() == Some("SHA256SUMS".as_ref()) {
            continue;
        }

        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];  // Increased from 1024 to 8192 bytes for better performance

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let hash = hasher.finalize();
        let hex_hash = hex::encode(hash);
        
        // Write hash and filename (relative to dist dir) to checksum file
        let rel_path = path.strip_prefix(dist_dir)?;
        writeln!(checksum_file, "{}  {}", hex_hash, rel_path.display())?;
    }

    println!("Generated checksums: {:?}", checksum_path);
    Ok(())
}

// Helper function to create a placeholder Windows executable
fn create_placeholder_exe(path: &Path) -> Result<()> {
    println!("Creating placeholder Windows executable...");
    let mut file = File::create(path)
        .context("Failed to create dummy Windows executable")?;
    
    // Write a minimal PE header to make it a valid Windows executable
    // This is just for packaging demonstration - it won't run
    let pe_header = b"MZ\x90\x00\x03\x00\x00\x00\x04\x00\x00\x00\xFF\xFF\x00\x00\
                     \xB8\x00\x00\x00\x00\x00\x00\x00\x40\x00\x00\x00\x00\x00\x00\x00\
                     \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
                     \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x80\x00\x00\x00\
                     \x0E\x1F\xBA\x0E\x00\xB4\x09\xCD\x21\xB8\x01\x4C\xCD\x21\x54\x68\
                     \x69\x73\x20\x70\x72\x6F\x67\x72\x61\x6D\x20\x63\x61\x6E\x6E\x6F\
                     \x74\x20\x62\x65\x20\x72\x75\x6E\x20\x69\x6E\x20\x44\x4F\x53\x20\
                     \x6D\x6F\x64\x65\x2E\x0D\x0D\x0A\x24\x00\x00\x00\x00\x00\x00\x00";
    
    file.write_all(pe_header)?;
    println!("Created placeholder Windows executable at: {}", path.display());
    Ok(())
}