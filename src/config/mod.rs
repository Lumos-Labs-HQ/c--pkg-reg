pub mod lockfile;
pub mod project;
pub mod resolver;

use crate::errors::CpkgResult;
use crate::types::{CpkgLock, CpkgToml};

#[allow(dead_code)]
pub struct ConfigBundle {
    pub project: CpkgToml,
    pub lockfile: Option<CpkgLock>,
    pub project_dir: String,
}

#[allow(dead_code)]
pub fn load_bundle(project_dir: &str) -> CpkgResult<ConfigBundle> {
    let config_path = project::find_config_path(project_dir)?;
    let project = project::load_config(&config_path)?;
    let lockfile_path = lockfile::lockfile_path(project_dir);
    let lockfile = if lockfile_path.exists() {
        Some(lockfile::load_lockfile(&lockfile_path)?)
    } else {
        None
    };
    Ok(ConfigBundle {
        project,
        lockfile,
        project_dir: project_dir.to_string(),
    })
}
