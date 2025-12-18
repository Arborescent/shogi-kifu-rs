//! CSA format parsers for different versions
//!
//! Each version is a separate parser with its own grammar.
//! All parsers output to the common `crate::value::GameRecord` type.

pub mod v2;
pub mod v2_1;
pub mod v2_2;
pub mod v3;

use crate::value::GameRecord;

/// CSA format version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    V2,
    V2_1,
    V2_2,
    V3,
}

/// Detect the CSA version from the input string
pub fn detect_version(input: &str) -> Option<Version> {
    for line in input.lines() {
        let trimmed = line.trim();

        // V3 encoding declaration
        if trimmed.starts_with("'CSA encoding=") {
            return Some(Version::V3);
        }

        // Skip comments
        if trimmed.starts_with('\'') {
            continue;
        }

        // Check version line
        if trimmed.starts_with('V') {
            if trimmed == "V3.0" {
                return Some(Version::V3);
            } else if trimmed == "V2.2" {
                return Some(Version::V2_2);
            } else if trimmed == "V2.1" {
                return Some(Version::V2_1);
            } else if trimmed == "V2" {
                return Some(Version::V2);
            }
        }

        // Non-comment, non-version line without finding version = unsupported
        if !trimmed.is_empty() {
            return None;
        }
    }
    None
}

/// Parse error type
#[derive(Debug)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CSA parse error: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

/// Parse a CSA file, auto-detecting the version
pub fn parse(input: &str) -> Result<GameRecord, ParseError> {
    let version = detect_version(input)
        .ok_or_else(|| ParseError("No version found or unsupported version".to_string()))?;

    match version {
        Version::V2 => v2::parse(input).map_err(|e| ParseError(e.0)),
        Version::V2_1 => v2_1::parse(input).map_err(|e| ParseError(e.0)),
        Version::V2_2 => v2_2::parse(input).map_err(|e| ParseError(e.0)),
        Version::V3 => v3::parse(input).map_err(|e| ParseError(e.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_csa;

    #[test]
    fn test_detect_version_v2() {
        assert_eq!(detect_version("V2\nPI\n+\n"), Some(Version::V2));
    }

    #[test]
    fn test_detect_version_v2_1() {
        assert_eq!(detect_version("V2.1\nPI\n+\n"), Some(Version::V2_1));
    }

    #[test]
    fn test_detect_version_v2_2() {
        assert_eq!(detect_version("V2.2\nPI\n+\n"), Some(Version::V2_2));
    }

    #[test]
    fn test_detect_version_v3() {
        assert_eq!(detect_version("V3.0\nPI\n+\n"), Some(Version::V3));
    }

    #[test]
    fn test_detect_version_v3_with_encoding() {
        assert_eq!(
            detect_version("'CSA encoding=UTF-8\nV3.0\nPI\n+\n"),
            Some(Version::V3)
        );
    }

    #[test]
    fn test_detect_version_no_version() {
        assert_eq!(detect_version("PI\n+\n"), None);
    }

    #[test]
    fn test_tsumi_without_trailing_newline() {
        // Test that %TSUMI at end of file (no trailing newline) is parsed correctly
        let csa = "V2.2\nPI\n+\n+7776FU\n%TSUMI";
        let result = parse_csa(csa);
        assert!(result.is_ok(), "Failed to parse CSA: {:?}", result.err());
        let record = result.unwrap();
        assert_eq!(record.moves.len(), 2, "Expected 2 moves (1 normal + 1 TSUMI)");
        assert!(
            matches!(record.moves[1].action, crate::Action::Tsumi),
            "Expected Action::Tsumi, got {:?}",
            record.moves[1].action
        );
    }

    #[test]
    fn test_tsumi_with_trailing_newline() {
        // Test that %TSUMI with trailing newline is also parsed correctly
        let csa = "V2.2\nPI\n+\n+7776FU\n%TSUMI\n";
        let result = parse_csa(csa);
        assert!(result.is_ok(), "Failed to parse CSA: {:?}", result.err());
        let record = result.unwrap();
        assert_eq!(record.moves.len(), 2, "Expected 2 moves (1 normal + 1 TSUMI)");
        assert!(
            matches!(record.moves[1].action, crate::Action::Tsumi),
            "Expected Action::Tsumi, got {:?}",
            record.moves[1].action
        );
    }
}
