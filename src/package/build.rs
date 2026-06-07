use crate::errors::CpkgResult;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn build_package(source_dir: &Path) -> CpkgResult<()> {
    log::info!("Building package in {}", source_dir.display());

    let cmake_lists = source_dir.join("CMakeLists.txt");
    let makefile = source_dir.join("Makefile");

    if cmake_lists.exists() {
        build_cmake(source_dir)
    } else if makefile.exists() {
        build_make(source_dir)
    } else {
        log::info!("No build system detected — treating as header-only");
        Ok(())
    }
}

fn build_cmake(source_dir: &Path) -> CpkgResult<()> {
    let build_dir = source_dir.join("build");
    std::fs::create_dir_all(&build_dir)?;

    let cmake_output = Command::new("cmake")
        .args(["-S", &source_dir.to_string_lossy(), "-B", &build_dir.to_string_lossy()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| crate::errors::CpkgError::BuildError(
            source_dir.display().to_string(),
            e.to_string(),
        ))?;

    if !cmake_output.status.success() {
        let stderr = String::from_utf8_lossy(&cmake_output.stderr);
        return Err(crate::errors::CpkgError::BuildError(
            source_dir.display().to_string(),
            format!("cmake configure failed: {}", stderr.lines().last().unwrap_or("")),
        ));
    }

    let parallelism = std::thread::available_parallelism()
        .map(|n| n.get().to_string())
        .unwrap_or_else(|_| "1".to_string());

    let build_output = Command::new("cmake")
        .args(["--build", &build_dir.to_string_lossy(), "--", &format!("-j{}", parallelism)])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| crate::errors::CpkgError::BuildError(
            source_dir.display().to_string(),
            e.to_string(),
        ))?;

    if !build_output.status.success() {
        let stderr = String::from_utf8_lossy(&build_output.stderr);
        return Err(crate::errors::CpkgError::BuildError(
            source_dir.display().to_string(),
            format!("cmake build failed: {}", stderr.lines().last().unwrap_or("")),
        ));
    }

    Ok(())
}

fn build_make(source_dir: &Path) -> CpkgResult<()> {
    let parallelism = std::thread::available_parallelism()
        .map(|n| format!("-j{}", n.get()))
        .unwrap_or_else(|_| "-j1".to_string());

    let output = Command::new("make")
        .arg("-C")
        .arg(source_dir)
        .arg(&parallelism)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| crate::errors::CpkgError::BuildError(
            source_dir.display().to_string(),
            e.to_string(),
        ))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::errors::CpkgError::BuildError(
            source_dir.display().to_string(),
            format!("make failed: {}", stderr.lines().last().unwrap_or("")),
        ));
    }

    Ok(())
}
