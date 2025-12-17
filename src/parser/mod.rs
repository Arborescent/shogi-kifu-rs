pub mod csa;

use std::error::Error;
use std::fmt;

use crate::value::GameRecord;

#[derive(Debug)]
pub enum CsaError {
    ParseError(String),
}

impl fmt::Display for CsaError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CsaError::ParseError(msg) => write!(f, "failed to parse: {}", msg),
        }
    }
}

impl Error for CsaError {}

////////////////////////////////////////////////////////////////////////////////

/// Parse a CSA file with automatic version detection.
pub fn parse_csa(s: &str) -> Result<GameRecord, CsaError> {
    csa::parse(s).map_err(|e| CsaError::ParseError(e.0))
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::{self, File};
    use std::io::Read;
    use std::path::Path;

    #[test]
    fn load_fixtures() {
        let fixtures_dir = Path::new("fixtures/");
        let dir_entries = fs::read_dir(fixtures_dir).unwrap();

        for entry in dir_entries {
            let path = entry.unwrap().path();
            if !path.is_file() {
                continue;
            }

            let filename = path.file_name().unwrap().to_str().unwrap();

            let mut file = File::open(&path).unwrap();
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .expect("failed to load a fixture content");
            let res = parse_csa(&contents);

            // v1.csa has no version line - we don't support versionless files
            if filename == "v1.csa" {
                assert!(res.is_err(), "v1.csa should fail (no version)");
            } else {
                assert!(res.is_ok(), "Failed to parse {:?}: {:?}", path, res);
            }
        }
    }
}
