use crate::errors::{CpkgError, CpkgResult};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;

fn hash_file(path: &Path) -> CpkgResult<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn verify_single(path: &Path, expected_sha256: &str) -> CpkgResult<()> {
    let result = hash_file(path)?;
    if result != expected_sha256 {
        return Err(CpkgError::ChecksumMismatch(expected_sha256.to_string(), result));
    }
    Ok(())
}

#[allow(dead_code)]
pub fn compute(path: &Path) -> CpkgResult<String> {
    hash_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn tmp_file(data: &[u8]) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(data).unwrap();
        f
    }

    #[test]
    fn compute_is_stable() {
        let f = tmp_file(b"hello cpkg");
        let h1 = compute(f.path()).unwrap();
        let h2 = compute(f.path()).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn compute_streams_large_file() {
        // 200 KB > 64 KB buffer — exercises multi-chunk path
        let f = tmp_file(&vec![0xABu8; 200 * 1024]);
        let h = compute(f.path()).unwrap();
        assert_eq!(h.len(), 64);
    }

    #[test]
    fn verify_single_pass_and_fail() {
        let f = tmp_file(b"test data");
        let correct = compute(f.path()).unwrap();
        assert!(verify_single(f.path(), &correct).is_ok());
        assert!(verify_single(f.path(), "000000").is_err());
    }
}
