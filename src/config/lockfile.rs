use crate::errors::{CpkgError, CpkgResult};
use crate::types::CpkgLock;
use std::fs;
use std::path::{Path, PathBuf};

pub fn lockfile_path(project_dir: &str) -> PathBuf {
    Path::new(project_dir).join("cpkg.lock")
}

pub fn load_lockfile(path: &Path) -> CpkgResult<CpkgLock> {
    let content = fs::read_to_string(path)?;
    let lockfile: CpkgLock = toml::from_str(&content)
        .map_err(|e| CpkgError::ConfigParseError("cpkg.lock".to_string(), e.to_string()))?;
    Ok(lockfile)
}

pub fn save_lockfile(path: &Path, lockfile: &CpkgLock) -> CpkgResult<()> {
    let content = toml::to_string_pretty(lockfile)
        .map_err(|e| CpkgError::SerializeError("cpkg.lock".to_string(), e.to_string()))?;
    fs::write(path, content)?;
    Ok(())
}
