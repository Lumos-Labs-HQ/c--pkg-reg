pub mod build;
pub mod extract;
pub mod fetch;
pub mod link;
pub mod verify;

use crate::config::{lockfile, project};
use crate::errors::CpkgResult;
use crate::paths::{self, CpkgLayout};
use crate::types::{CpkgLock, Dependency, LockMetadata, LockedPackage};

pub fn install(name: &str, global: bool, version_override: Option<&str>) -> CpkgResult<()> {
    let (layout, _target) = paths::resolve_target(global)?;
    layout.ensure_dirs()?;

    let version_constraint = version_override.unwrap_or("*");
    let dep = Dependency {
        name: name.to_string(),
        version: version_constraint.to_string(),
        global,
    };

    let resolved = crate::config::resolver::resolve_dependencies(&[dep], None)?;
    let pkg = resolved
        .first()
        .ok_or(crate::errors::CpkgError::PackageNotFound(name.to_string()))?;

    let version_str = pkg.version.to_string();
    let urls = package_download_urls(pkg);
    let tarball = layout.cache_dir().join(format!("{}-{}.tar.gz", name, &version_str));

    let mut actual_hash = String::new();
    let mut last_error = String::new();
    for url in &urls {
        match fetch::download(url, &tarball) {
            Ok(hash) => { actual_hash = hash; break; }
            Err(e) => { last_error = e.to_string(); }
        }
    }
    if actual_hash.is_empty() {
        return Err(crate::errors::CpkgError::DownloadError(
            format!("{}-{}", name, version_str),
            last_error,
        ));
    }

    if let Some(ref expected_hash) = pkg.sha256 {
        if &actual_hash != expected_hash {
            return Err(crate::errors::CpkgError::ChecksumMismatch(
                expected_hash.clone(),
                actual_hash,
            ));
        }
    }

    let install_dir = layout.package_install_dir(name, &version_str);
    if !install_dir.exists() {
        extract::extract_tarball(&tarball, &install_dir)?;
    }

    if let Err(e) = build::build_package(&install_dir) {
        log::warn!("Build skipped: {}", e);
    }

    let strategy = paths::link_strategy();
    if let Some(src) = find_include_dir(&install_dir) {
        link_include_tree(&src, &layout.include_dir(), name, strategy)?;
    } else {
        log::warn!("No headers found in {}", install_dir.display());
    }

    let lib_src = pkg.lib_dir.as_ref()
        .map(|d| install_dir.join(d))
        .filter(|p| p.exists())
        .or_else(|| find_lib_dir(&install_dir));

    if let Some(lib_dir) = lib_src {
        link::link_libraries(&lib_dir, &layout.lib_dir(), strategy)?;
    }

    if !global {
        update_config(name, version_constraint)?;
    }

    update_lockfile(name, pkg, global)?;

    println!("Installed {} v{} ({})", name, version_str, if global { "global" } else { "local" });
    Ok(())
}

pub fn uninstall(name: &str) -> CpkgResult<()> {
    let config_path = project::find_config_path(".")?;
    let mut config = project::load_config(&config_path)?;

    let before = config.dependencies.len();
    config.dependencies.retain(|d| d.name != name);
    if config.dependencies.len() == before {
        println!("Package '{}' not found in cpkg.toml", name);
        return Ok(());
    }

    project::save_config(&config_path, &config)?;

    let layout = CpkgLayout::local(".");
    let include_dst = layout.include_dir().join(name);
    if include_dst.exists() {
        if include_dst.is_symlink() || include_dst.is_file() {
            std::fs::remove_file(&include_dst)?;
        } else {
            std::fs::remove_dir_all(&include_dst)?;
        }
    }

    println!("Removed {}", name);
    Ok(())
}

fn update_config(name: &str, version: &str) -> CpkgResult<()> {
    let config_path = project::find_config_path(".")?;
    let mut config = project::load_config(&config_path)?;

    config.dependencies.retain(|d| d.name != name);
    config.dependencies.push(Dependency {
        name: name.to_string(),
        version: version.to_string(),
        global: false,
    });

    project::save_config(&config_path, &config)?;
    Ok(())
}

fn update_lockfile(name: &str, pkg: &LockedPackage, global: bool) -> CpkgResult<()> {
    let lock_path = lockfile::lockfile_path(".");
    let mut lock = if lock_path.exists() {
        lockfile::load_lockfile(&lock_path)?
    } else {
        CpkgLock {
            metadata: LockMetadata {
                lockfile_version: 1,
                generated_by: "cpkg 0.2.0".to_string(),
            },
            packages: vec![],
        }
    };

    lock.packages.retain(|p| p.name != name);

    let mut locked = pkg.clone();
    if global {
        locked.source = String::from("global");
    }
    lock.packages.push(locked);

    lockfile::save_lockfile(&lock_path, &lock)?;
    Ok(())
}

fn package_download_urls(pkg: &LockedPackage) -> Vec<String> {
    let repo_part = pkg.source.trim_start_matches("github.com/");
    let ver = pkg.version.to_string();
    vec![
        format!("https://github.com/{}/archive/refs/tags/v{}.tar.gz", repo_part, ver),
        format!("https://github.com/{}/archive/refs/tags/{}.tar.gz", repo_part, ver),
    ]
}

/// Single walkdir pass — returns (include_dir, lib_dir) without scanning twice.
fn find_dirs(pkg_dir: &std::path::Path) -> (Option<std::path::PathBuf>, Option<std::path::PathBuf>) {
    use std::ffi::OsStr;

    // Fast path: standard layout
    let inc = pkg_dir.join("include");
    let lib = pkg_dir.join("lib");
    if inc.exists() && lib.exists() {
        return (Some(inc), Some(lib));
    }

    let mut include_dir: Option<std::path::PathBuf> = if inc.exists() { Some(inc) } else { None };
    let mut lib_dir: Option<std::path::PathBuf> = if lib.exists() { Some(lib) } else { None };

    if include_dir.is_some() && lib_dir.is_some() {
        return (include_dir, lib_dir);
    }

    for entry in walkdir::WalkDir::new(pkg_dir).max_depth(5).into_iter().flatten() {
        let path = entry.path();
        let ext = path.extension();

        if lib_dir.is_none() && (ext == Some(OsStr::new("a")) || ext == Some(OsStr::new("so"))) {
            lib_dir = path.parent().map(|p| p.to_path_buf());
        }

        if include_dir.is_none() && path.is_file()
            && (ext == Some(OsStr::new("h")) || ext == Some(OsStr::new("hpp")))
        {
            // Use the nearest ancestor named "include", "src", or the pkg root
            let ancestor = path.ancestors().find(|a| {
                let name = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
                name == "include" || name == "src" || *a == pkg_dir
            });
            include_dir = ancestor.map(|p| p.to_path_buf());
        }

        if include_dir.is_some() && lib_dir.is_some() {
            break;
        }
    }

    (include_dir, lib_dir)
}

fn find_lib_dir(pkg_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    find_dirs(pkg_dir).1
}

fn find_include_dir(pkg_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    find_dirs(pkg_dir).0
}

fn link_include_tree(
    src: &std::path::Path,
    include_root: &std::path::Path,
    pkg_name: &str,
    strategy: crate::types::LinkStrategy,
) -> CpkgResult<()> {
    let entries: Vec<_> = std::fs::read_dir(src)?.filter_map(|e| e.ok()).collect();

    let has_direct_headers = entries.iter().any(|e| {
        let p = e.path();
        p.is_file() && matches!(p.extension().and_then(|x| x.to_str()), Some("h") | Some("hpp"))
    });

    if has_direct_headers {
        link::link_include(src, &include_root.join(pkg_name), strategy)?;
    } else {
        for entry in &entries {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().unwrap();
                link::link_include(&path, &include_root.join(dir_name), strategy)?;
            }
        }
    }
    Ok(())
}
