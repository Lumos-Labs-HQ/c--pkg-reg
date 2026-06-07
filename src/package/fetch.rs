use crate::errors::CpkgResult;
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};
use which::which;

/// Download `url` to `dest`, streaming directly to disk.
/// Returns the SHA-256 hex digest of the downloaded content.
pub fn download(url: &str, dest: &std::path::Path) -> CpkgResult<String> {
    if dest.exists() {
        log::info!("Already cached: {}", dest.display());
        return crate::package::verify::compute(dest);
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    log::info!("Downloading {} ...", url);
    let mut response = reqwest::blocking::get(url)
        .map_err(|e| crate::errors::CpkgError::DownloadError(url.to_string(), e.to_string()))?;

    if !response.status().is_success() {
        return Err(crate::errors::CpkgError::DownloadError(
            url.to_string(),
            format!("HTTP {}", response.status()),
        ));
    }

    let mut file = std::fs::File::create(dest)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = response.read(&mut buf)
            .map_err(|e| crate::errors::CpkgError::DownloadError(url.to_string(), e.to_string()))?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
        file.write_all(&buf[..n])?;
    }

    log::info!("Downloaded to {}", dest.display());
    Ok(format!("{:x}", hasher.finalize()))
}

#[allow(dead_code)]
pub fn find_executable(name: &str) -> Option<std::path::PathBuf> {
    which(name).ok()
}
