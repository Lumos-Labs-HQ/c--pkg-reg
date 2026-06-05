use crate::errors::{CpkgError, CpkgResult};
use std::fs;
use std::path::Path;

pub fn run(path_str: &str) -> CpkgResult<()> {
    let dir = Path::new(path_str);
    let config_path = dir.join("cpkg.toml");

    if config_path.exists() {
        println!("cpkg.toml already exists in {}", dir.display());
        return Ok(());
    }

    let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("unnamed");

    let content = format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\n\n[[scripts]]\nname = \"build\"\ncommand = \"g++ -std=c++17 src/main.cpp -I.cpkg/include -L.cpkg/lib -o app\"\n\n[[scripts]]\nname = \"run\"\ncommand = \"./app\"\n",
        dir_name
    );

    fs::create_dir_all(dir.join("src"))
        .map_err(|e| CpkgError::Io(e))?;

    fs::write(&config_path, content)?;
    println!("Created cpkg.toml in {}", dir.display());
    Ok(())
}
