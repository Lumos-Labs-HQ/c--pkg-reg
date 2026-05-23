use crate::errors::{CpkgError, CpkgResult};
use crate::types::{CpkgToml, PackageMeta};
use std::fs;
use std::path::Path;

pub fn run(path_str: &str) -> CpkgResult<()> {
    let dir = Path::new(path_str);
    let config_path = dir.join("cpkg.toml");

    if config_path.exists() {
        println!("cpkg.toml already exists in {}", dir.display());
        return Ok(());
    }

    let dir_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unnamed");

    let config = CpkgToml {
        package: PackageMeta {
            name: dir_name.to_string(),
            version: "0.1.0".to_string(),
            description: None,
            authors: None,
        },
        dependencies: vec![],
        scripts: vec![],
        compiler: None,
        cpp_standard: None,
    };

    let content = toml::to_string_pretty(&config)
        .map_err(|e| CpkgError::SerializeError("cpkg.toml".to_string(), e.to_string()))?;

    fs::write(&config_path, content)?;
    println!("Created cpkg.toml in {}", dir.display());

    Ok(())
}
