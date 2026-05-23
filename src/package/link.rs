use crate::errors::CpkgResult;
use crate::types::LinkStrategy;
use std::fs;
use std::path::Path;

pub fn link_include(
    package_include: &Path,
    target_include: &Path,
    strategy: LinkStrategy,
) -> CpkgResult<()> {
    if let Some(parent) = target_include.parent() {
        fs::create_dir_all(parent)?;
    }

    if target_include.exists() {
        if target_include.is_symlink() || !target_include.is_dir() {
            fs::remove_file(target_include)?;
        } else {
            fs::remove_dir_all(target_include)?;
        }
    }

    match strategy {
        LinkStrategy::Symlink => {
            #[cfg(target_family = "unix")]
            std::os::unix::fs::symlink(package_include, target_include)?;
            #[cfg(not(target_family = "unix"))]
            copy_recursive(package_include, target_include)?;
        }
        LinkStrategy::Copy => {
            copy_recursive(package_include, target_include)?;
        }
    }

    log::info!(
        "Linked include: {} -> {}",
        package_include.display(),
        target_include.display()
    );
    Ok(())
}

pub fn link_libraries(
    package_lib_dir: &Path,
    target_lib_dir: &Path,
    strategy: LinkStrategy,
) -> CpkgResult<()> {
    fs::create_dir_all(target_lib_dir)?;

    for entry in fs::read_dir(package_lib_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = path.file_name().unwrap();
        let target = target_lib_dir.join(file_name);

        if target.exists() {
            fs::remove_file(&target)?;
        }

        match strategy {
            LinkStrategy::Symlink => {
                #[cfg(target_family = "unix")]
                std::os::unix::fs::symlink(&path, &target)?;
                #[cfg(not(target_family = "unix"))]
                fs::copy(&path, &target)?;
            }
            LinkStrategy::Copy => {
                fs::copy(&path, &target)?;
            }
        }
    }

    log::info!("Linked libraries from {} to {}", package_lib_dir.display(), target_lib_dir.display());
    Ok(())
}

fn copy_recursive(src: &Path, dst: &Path) -> CpkgResult<()> {
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if file_type.is_dir() {
                copy_recursive(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
    }
    Ok(())
}
