use crate::errors::CpkgResult;
use which::which;

pub fn download(url: &str, dest: &std::path::Path) -> CpkgResult<()> {
    if dest.exists() {
        log::info!("Already cached: {}", dest.display());
        return Ok(());
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    log::info!("Downloading {} ...", url);
    let response = reqwest::blocking::get(url)
        .map_err(|e| crate::errors::CpkgError::DownloadError(url.to_string(), e.to_string()))?;

    if !response.status().is_success() {
        return Err(crate::errors::CpkgError::DownloadError(
            url.to_string(),
            format!("HTTP {}", response.status()),
        ));
    }

    let bytes = response
        .bytes()
        .map_err(|e| crate::errors::CpkgError::DownloadError(url.to_string(), e.to_string()))?;

    std::fs::write(dest, &bytes)?;
    log::info!("Downloaded to {}", dest.display());
    Ok(())
}

#[allow(dead_code)]
pub fn find_executable(name: &str) -> Option<std::path::PathBuf> {
    which(name).ok()
}
