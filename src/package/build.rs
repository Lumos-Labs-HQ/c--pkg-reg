use crate::errors::CpkgResult;
use std::path::Path;
use std::process::Command;

pub fn build_package(source_dir: &Path) -> CpkgResult<()> {
    log::info!("Building package in {}", source_dir.display());

    let cmake_lists = source_dir.join("CMakeLists.txt");
    let makefile = source_dir.join("Makefile");

    if cmake_lists.exists() {
        build_cmake(source_dir)
    } else if makefile.exists() {
        build_make(source_dir)
    } else {
        // No build system — header-only library, nothing to do
        log::info!("No build system detected — treating as header-only");
        Ok(())
    }
}

fn build_cmake(source_dir: &Path) -> CpkgResult<()> {
    let build_dir = source_dir.join("build");
    std::fs::create_dir_all(&build_dir)?;

    let cmake_status = Command::new("cmake")
        .args(["-S", &source_dir.to_string_lossy(), "-B", &build_dir.to_string_lossy()])
        .status()
        .map_err(|e| crate::errors::CpkgError::BuildError(
            source_dir.display().to_string(),
            e.to_string(),
        ))?;

    if !cmake_status.success() {
        return Err(crate::errors::CpkgError::BuildError(
            source_dir.display().to_string(),
            "cmake configure failed".to_string(),
        ));
    }

    let build_status = Command::new("cmake")
        .args(["--build", &build_dir.to_string_lossy()])
        .status()
        .map_err(|e| crate::errors::CpkgError::BuildError(
            source_dir.display().to_string(),
            e.to_string(),
        ))?;

    if !build_status.success() {
        return Err(crate::errors::CpkgError::BuildError(
            source_dir.display().to_string(),
            "cmake build failed".to_string(),
        ));
    }

    Ok(())
}

fn build_make(source_dir: &Path) -> CpkgResult<()> {
    let status = Command::new("make")
        .arg("-C")
        .arg(source_dir)
        .status()
        .map_err(|e| crate::errors::CpkgError::BuildError(
            source_dir.display().to_string(),
            e.to_string(),
        ))?;

    if !status.success() {
        return Err(crate::errors::CpkgError::BuildError(
            source_dir.display().to_string(),
            "make failed".to_string(),
        ));
    }

    Ok(())
}
