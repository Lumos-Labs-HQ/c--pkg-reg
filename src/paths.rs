use crate::errors::CpkgResult;
use crate::types::{InstallTarget, LinkStrategy};
use std::path::PathBuf;

pub struct CpkgLayout {
    pub root: PathBuf,
}

impl CpkgLayout {
    pub fn local(project_root: &str) -> Self {
        Self {
            root: PathBuf::from(project_root).join(".cpkg"),
        }
    }

    pub fn global() -> Self {
        let base = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("cpkg");
        Self { root: base }
    }

    pub fn packages_dir(&self) -> PathBuf {
        self.root.join("packages")
    }
    pub fn include_dir(&self) -> PathBuf {
        self.root.join("include")
    }
    pub fn lib_dir(&self) -> PathBuf {
        self.root.join("lib")
    }
    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }
    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    pub fn ensure_dirs(&self) -> CpkgResult<()> {
        std::fs::create_dir_all(self.packages_dir())?;
        std::fs::create_dir_all(self.include_dir())?;
        std::fs::create_dir_all(self.lib_dir())?;
        std::fs::create_dir_all(self.cache_dir())?;
        std::fs::create_dir_all(self.bin_dir())?;
        Ok(())
    }

    pub fn package_install_dir(&self, name: &str, version: &str) -> PathBuf {
        self.packages_dir().join(format!("{}-{}", name, version))
    }
}

pub fn resolve_target(global: bool) -> CpkgResult<(CpkgLayout, InstallTarget)> {
    if global {
        let layout = CpkgLayout::global();
        Ok((layout, InstallTarget::Global))
    } else {
        let cwd = std::env::current_dir()?;
        let layout = CpkgLayout::local(cwd.to_str().unwrap_or("."));
        Ok((layout, InstallTarget::Local(cwd)))
    }
}

pub fn link_strategy() -> LinkStrategy {
    #[cfg(target_family = "unix")]
    {
        LinkStrategy::Symlink
    }
    #[cfg(not(target_family = "unix"))]
    {
        LinkStrategy::Copy
    }
}
