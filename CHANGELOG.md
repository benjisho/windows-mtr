# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2026-02-28

### Added
- Embedded Trippy runtime inside `mtr.exe`, removing the external `trip.exe` runtime dependency.
- Added JSON output flags: `-j/--json` and `--json-pretty`.
- Added Linux-parity/high-impact flags: `-b/--show-asn`, `-s/--packet-size`, `-S/--src`, `-z`, `--ecmp`, `-w/--report-wide`, `--interface`, and `--source-port`.
- Added DNS cache TTL control with `--dns-cache-ttl`.
- Added power-user passthrough with `--trippy-flags "..."`.
- Added Linux mtr vs windows-mtr vs trippy mapping table in `USAGE.md`.

### Changed
- Updated to Rust 2024 edition.
- Updated Trippy integration to `trippy`/`trippy-tui` 0.13.0.
- Updated documentation and roadmap to reflect delivered v2.0 capabilities.

### Fixed
- Hardened security workflow with a pinned Rust toolchain, explicit `cargo-deny` policy file, and an always-on GitHub Actions job summary.
- Tuned `cargo-deny` policy to handle known transitive advisories and unavoidable duplicate crates in the Trippy 0.13 dependency graph.
- Added `cargo-audit` policy configuration with documented temporary ignores for transitive upstream advisories in the current Trippy 0.13 stack.
- Fixed JSON output behavior: suppress banner in JSON modes and make `--json` emit compact JSON while `--json-pretty` preserves pretty formatting.

## [1.1.2] - 2025-05-04

### Fixed
- Fixed TCP/UDP mode with correct port parameter handling
- Corrected command-line argument translation for trippy 0.12.2

## [1.1.1] - 2025-05-04

### Fixed
- Fixed CI release pipeline triggers
- Minor improvements to error handling and reporting
- Additional validation for command-line arguments

## [1.1.0] - 2025-05-04

### Added
- Enhanced command-line argument handling for better user experience
- Improved compatibility with trippy 0.12.2 protocol specification
- Better error detection and messaging for administrator privileges

### Fixed
- Fixed TCP/UDP mode with port specifications
- Updated protocol parameter handling to use the new format required by trippy
- Corrected documentation to accurately reflect administrator requirements
- Fixed port validation and parameter checking

## [1.0.5] - 2025-05-04

### Added
- Clearer error message for administrator privilege requirements
- Detailed CLI help with usage examples
- Improved error handling for command-line arguments

### Fixed
- Fixed command-line argument translation for TCP/UDP with port numbers
- Updated protocol parameter handling for compatibility with trippy 0.12.2
- Improved error detection for privilege-related errors
- Fixed documentation to correctly indicate administrator privileges are required

## [1.0.0] - 2025-04-30

### Added --> [1.0.0]

- MSI installer for system-wide installation
- Improved GitHub Actions release workflow with cargo-dist integration
- Enhanced documentation with detailed installation instructions
- XZ compression for smaller distribution packages
- Comprehensive test suite with 35 integration scenarios and fuzzing

### Changed

- Optimized memory usage with arena allocations and lock-free data structures
- Improved error handling with thiserror and anyhow
- Enhanced Windows compatibility with direct WinAPI access
- RFC 4884 compliant ICMP extension headers

### Fixed

- Packet fragmentation handling on Windows
- DNS resolution edge cases with IDN support
- Timing precision for accurate packet RTT measurement

## [0.1.0] - 2025-04-29

### Added -> [0.1.0]

- Initial release of Windows MTR
- ICMP, TCP, and UDP probe support
- Report mode (-r) that produces output identical to Linux mtr
- Count (-c), interval (-i), and timeout (-w) options
- Rich TUI interface for live monitoring
- Windows binary packaging via xtask
