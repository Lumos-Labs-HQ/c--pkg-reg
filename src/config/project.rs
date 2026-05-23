use crate::errors::{CpkgError, CpkgResult};
use crate::types::CpkgToml;
use std::fs;
use std::path::{Path, PathBuf};

pub fn find_config_path(dir: &str) -> CpkgResult<PathBuf> {
    let path = Path::new(dir).join("cpkg.toml");
    if !path.exists() {
        return Err(CpkgError::ConfigNotFound(dir.to_string()));
    }
    Ok(path)
}

pub fn load_config(path: &Path) -> CpkgResult<CpkgToml> {
    let content = fs::read_to_string(path)?;
    let config: CpkgToml = toml::from_str(&content)
        .map_err(|e| CpkgError::ConfigParseError(path.display().to_string(), e.to_string()))?;
    Ok(config)
}

pub fn save_config(path: &Path, config: &CpkgToml) -> CpkgResult<()> {
    let content = toml::to_string_pretty(config)
        .map_err(|e| CpkgError::SerializeError("cpkg.toml".to_string(), e.to_string()))?;
    fs::write(path, content)?;
    Ok(())
}
