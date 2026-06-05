use crate::errors::CpkgResult;
use crate::types::{Compiler, CompilerName};
use std::process::Command;
use which::which;

#[cfg(target_family = "unix")]
const CANDIDATES: &[(&str, fn() -> CompilerName)] = &[
    ("g++",     || CompilerName::Gcc),
    ("clang++", || CompilerName::Clang),
];

#[cfg(target_family = "windows")]
const CANDIDATES: &[(&str, fn() -> CompilerName)] = &[
    ("cl.exe",      || CompilerName::Msvc),
    ("g++.exe",     || CompilerName::Gcc),
    ("clang++.exe", || CompilerName::Clang),
];

pub fn detect_all() -> CpkgResult<Vec<Compiler>> {
    let mut found = Vec::new();
    for (binary, name_fn) in CANDIDATES {
        if let Ok(path) = which(binary) {
            if let Ok(version) = query_version(&path) {
                found.push(Compiler { name: name_fn(), path, version });
            }
        }
    }
    Ok(found)
}

#[allow(dead_code)]
pub fn resolve(preference: Option<&str>) -> CpkgResult<Compiler> {
    let compilers = detect_all()?;
    match preference {
        Some(pref) => compilers.into_iter()
            .find(|c| c.name.display().to_lowercase().contains(&pref.to_lowercase()))
            .ok_or(crate::errors::CpkgError::CompilerNotFound(pref.to_string())),
        None => compilers.into_iter().next()
            .ok_or(crate::errors::CpkgError::NoCompilerFound),
    }
}

fn query_version(path: &std::path::Path) -> CpkgResult<crate::types::CompilerVersion> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|e| crate::errors::CpkgError::VersionParseError(e.to_string()))?;

    let combined = format!(
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_version_string(&combined)
}

fn parse_version_string(raw: &str) -> CpkgResult<crate::types::CompilerVersion> {
    use regex::Regex;
    let re = Regex::new(r"(\d+)\.(\d+)\.(\d+)").expect("hardcoded regex should be valid");
    let caps = re.captures(raw)
        .ok_or_else(|| crate::errors::CpkgError::VersionParseError(raw.to_string()))?;

    Ok(crate::types::CompilerVersion {
        major: caps[1].parse().unwrap_or(0),
        minor: caps[2].parse().unwrap_or(0),
        patch: caps[3].parse().unwrap_or(0),
        raw: raw.trim().to_string(),
    })
}
