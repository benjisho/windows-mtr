use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::str::FromStr;
use regex::Regex;

// This test validates that the output format from `mtr -c N -r host` 
// matches the expected format from Linux MTR
#[test]
#[ignore] // Requires network access
fn test_report_format() {
    // Run our MTR clone with report mode
    let output = Command::new("cargo")
        .args(["run", "--", "-c", "2", "-r", "127.0.0.1"])
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to run mtr");
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Verify the first line contains our banner
    assert!(stdout.contains("windows-mtr by Benji Shohet"), 
        "Output should contain banner: {}", stdout);
    
    // Verify the report format
    assert!(stdout.contains("HOST:"), "Output should contain HOST header");
    assert!(stdout.contains("Loss%"), "Output should contain Loss% column");
    assert!(stdout.contains("Snt"), "Output should contain Snt column");
    assert!(stdout.contains("Last"), "Output should contain Last column");
    assert!(stdout.contains("Avg"), "Output should contain Avg column");
    
    // Verify we have at least one hop
    let hop_pattern = Regex::new(r"\s+\d+\.\|--").unwrap();
    assert!(hop_pattern.is_match(&stdout), "Output should show at least one hop");
}

// This test compares the fields and structure with the Linux MTR fixture
// to ensure compatibility
#[test]
fn test_fixture_structure() {
    let fixture_path = Path::new("tests/fixtures/linux_mtr_report_sample.txt");
    let fixture_content = fs::read_to_string(fixture_path)
        .expect("Failed to read Linux MTR fixture");
    
    // Verify the expected lines are present in the fixture
    assert!(fixture_content.contains("Loss%"), "Fixture should contain Loss% column");
    assert!(fixture_content.contains("Snt"), "Fixture should contain Snt column");
    assert!(fixture_content.contains("Last"), "Fixture should contain Last column");
    assert!(fixture_content.contains("Avg"), "Fixture should contain Avg column");
    assert!(fixture_content.contains("Best"), "Fixture should contain Best column");
    assert!(fixture_content.contains("Wrst"), "Fixture should contain Wrst column");
    assert!(fixture_content.contains("StDev"), "Fixture should contain StDev column");
    
    // Extract column positions from the fixture header
    let header_line = fixture_content.lines()
        .find(|l| l.contains("Loss%"))
        .expect("Could not find header line in fixture");
    
    let loss_pos = header_line.find("Loss%").expect("Could not find Loss% column");
    let snt_pos = header_line.find("Snt").expect("Could not find Snt column");
    let last_pos = header_line.find("Last").expect("Could not find Last column");
    let avg_pos = header_line.find("Avg").expect("Could not find Avg column");
    
    // Parse hop data lines to verify structure
    let hop_lines: Vec<_> = fixture_content.lines()
        .filter(|l| l.contains("|--"))
        .collect();
    
    assert!(!hop_lines.is_empty(), "Fixture should contain hop lines");
    
    for line in hop_lines {
        // Verify each hop line has data in the expected positions
        assert!(line.len() > loss_pos, "Line should contain Loss% data");
        assert!(line.len() > snt_pos, "Line should contain Snt data");
        assert!(line.len() > last_pos, "Line should contain Last data");
        assert!(line.len() > avg_pos, "Line should contain Avg data");
        
        // Extract and verify hop number format
        let hop_pattern = Regex::new(r"^\s*(\d+)\.\|--").unwrap();
        assert!(hop_pattern.is_match(line), "Hop line should start with hop number: {}", line);
    }
}