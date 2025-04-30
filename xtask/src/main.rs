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

fn main() -> Result<()> {
    let task = env::args().nth(1);
    match task.as_deref() {
        Some("dist") => dist()?,
        _ => {
            println!("Available tasks:");
            println!("  dist - Build release binaries and package for distribution");
        }
    }
    Ok(())
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
        if let Ok(mut file) = File::open(file_name) {
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;
            
            zip.start_file(file_name, options)?;
            zip.write_all(&contents)?;
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
        let mut buffer = [0; 1024];

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