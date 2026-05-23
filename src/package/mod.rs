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

    // Construct download URL
    let url = package_download_url(pkg);

    // Download
    let tarball = layout.cache_dir().join(format!("{}-{}.tar.gz", name, &version_str));
    fetch::download(&url, &tarball)?;

    // Verify checksum (if available)
    if let Some(ref expected_hash) = pkg.sha256 {
        verify::verify_single(&tarball, expected_hash)?;
    }

    // Extract
    let install_dir = layout.package_install_dir(name, &version_str);
    if !install_dir.exists() {
        extract::extract_tarball(&tarball, &install_dir)?;
    }

    // Build the package
    build::build_package(&install_dir)?;

    // Link includes
    let strategy = paths::link_strategy();
    for inc_dir in &pkg.include_dirs {
        let src = install_dir.join(inc_dir);
        let dst = layout.include_dir().join(name);
        if src.exists() {
            link::link_include(&src, &dst, strategy)?;
        }
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

fn package_download_url(pkg: &LockedPackage) -> String {
    match pkg.name.as_str() {
        "fmt" => format!(
            "https://github.com/fmtlib/fmt/archive/refs/tags/{}.tar.gz",
            pkg.version
        ),
        _ => format!(
            "https://github.com/{0}/{0}/archive/refs/tags/{1}.tar.gz",
            pkg.name, pkg.version,
        ),
    }
}
