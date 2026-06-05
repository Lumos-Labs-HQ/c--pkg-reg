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

    // Resolve version
    let resolved = crate::config::resolver::resolve_dependencies(&[dep], None)?;
    let pkg = resolved
        .first()
        .ok_or(crate::errors::CpkgError::PackageNotFound(name.to_string()))?;

    let version_str = pkg.version.to_string();

    // Construct download URL (try v-prefixed and bare tag formats)
    let urls = package_download_urls(pkg);
    let tarball = layout.cache_dir().join(format!("{}-{}.tar.gz", name, &version_str));

    let mut downloaded = false;
    let mut last_error = String::new();
    for url in &urls {
        match fetch::download(url, &tarball) {
            Ok(()) => { downloaded = true; break; }
            Err(e) => { last_error = e.to_string(); }
        }
    }
    if !downloaded {
        return Err(crate::errors::CpkgError::DownloadError(
            format!("{}-{}", name, version_str),
            last_error,
        ));
    }

    // Verify checksum (if available)
    if let Some(ref expected_hash) = pkg.sha256 {
        verify::verify_single(&tarball, expected_hash)?;
    }

    // Extract
    let install_dir = layout.package_install_dir(name, &version_str);
    if !install_dir.exists() {
        extract::extract_tarball(&tarball, &install_dir)?;
    }

    // Build the package (non-fatal — header-only libs don't need it)
    if let Err(e) = build::build_package(&install_dir) {
        log::warn!("Build skipped: {}", e);
    }

    // Link includes — auto-detect if standard "include/" doesn't exist
    let strategy = paths::link_strategy();
    let dst = layout.include_dir().join(name);
    if let Some(src) = find_include_dir(&install_dir) {
        link::link_include(&src, &dst, strategy)?;
    } else {
        log::warn!("No headers found in {}", install_dir.display());
    }

    // Link libraries
    if let Some(ref lib_dir) = pkg.lib_dir {
        let pkg_lib = install_dir.join(lib_dir);
        if pkg_lib.exists() {
            link::link_libraries(&pkg_lib, &layout.lib_dir(), strategy)?;
        }
    }

    // Update cpkg.toml (local only)
    if !global {
        update_config(name, version_constraint)?;
    }

    // Update cpkg.lock
    update_lockfile(name, pkg, global)?;

    println!(
        "Installed {} v{} ({})",
        name,
        version_str,
        if global { "global" } else { "local" }
    );
    Ok(())
}

pub fn uninstall(name: &str) -> CpkgResult<()> {
    // Load config
    let config_path = project::find_config_path(".")?;
    let mut config = project::load_config(&config_path)?;

    // Remove from dependencies
    let before = config.dependencies.len();
    config.dependencies.retain(|d| d.name != name);
    if config.dependencies.len() == before {
        println!("Package '{}' not found in cpkg.toml", name);
        return Ok(());
    }

    project::save_config(&config_path, &config)?;

    // Remove from .cpkg/
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

    // Replace or add dependency
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
                generated_by: "cpkg 0.1.0".to_string(),
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
    // source format: "github.com/{owner}/{repo}"
    let repo_part = pkg.source.trim_start_matches("github.com/");
    let ver = pkg.version.to_string();
    // Try v-prefixed first, then bare — some repos use v1.2.3, others 1.2.3
    vec![
        format!("https://github.com/{}/archive/refs/tags/v{}.tar.gz", repo_part, ver),
        format!("https://github.com/{}/archive/refs/tags/{}.tar.gz", repo_part, ver),
    ]
}

fn find_include_dir(pkg_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    // 1. Standard "include/" directory
    let include = pkg_dir.join("include");
    if include.exists() && include.is_dir() {
        return Some(include);
    }
    // 2. "src/" directory if it contains .h/.hpp files
    let src = pkg_dir.join("src");
    if src.exists() && src.is_dir() && dir_has_headers(&src) {
        return Some(src);
    }
    // 3. Scan top-level for any directory with .h/.hpp files
    if let Ok(entries) = std::fs::read_dir(pkg_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.file_name().map_or(false, |n| n != "build") {
                if dir_has_headers(&path) {
                    return Some(path);
                }
            }
        }
    }
    // 4. Root has headers directly — link the whole package dir
    if dir_has_headers(pkg_dir) {
        return Some(pkg_dir.to_path_buf());
    }
    None
}

fn dir_has_headers(dir: &std::path::Path) -> bool {
    use std::ffi::OsStr;
    walkdir::WalkDir::new(dir)
        .max_depth(4)
        .into_iter()
        .flatten()
        .any(|e| {
            e.path().extension() == Some(OsStr::new("h"))
                || e.path().extension() == Some(OsStr::new("hpp"))
        })
}
