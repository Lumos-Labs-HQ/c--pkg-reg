use crate::errors::{CpkgError, CpkgResult};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;

pub fn verify_single(path: &Path, expected_sha256: &str) -> CpkgResult<()> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    let result = format!("{:x}", hasher.finalize());

    if result != expected_sha256 {
        return Err(CpkgError::ChecksumMismatch(
            expected_sha256.to_string(),
            result,
        ));
    }

    Ok(())
}

#[allow(dead_code)]
pub fn compute(path: &Path) -> CpkgResult<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    Ok(format!("{:x}", hasher.finalize()))
}
