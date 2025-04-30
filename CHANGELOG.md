# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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