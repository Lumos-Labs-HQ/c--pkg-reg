use crate::errors::CpkgResult;

#[allow(dead_code)]
pub fn query(path: &std::path::Path) -> CpkgResult<crate::types::CompilerVersion> {
    use regex::Regex;
    use std::process::Command;

    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|e| crate::errors::CpkgError::VersionParseError(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{} {}", stdout, stderr);

    let re = Regex::new(r"(\d+)\.(\d+)\.(\d+)")
        .expect("hardcoded regex should be valid");

    let caps = re.captures(&combined)
        .ok_or_else(|| crate::errors::CpkgError::VersionParseError(combined.to_string()))?;

    Ok(crate::types::CompilerVersion {
        major: caps[1].parse().unwrap_or(0),
        minor: caps[2].parse().unwrap_or(0),
        patch: caps[3].parse().unwrap_or(0),
        raw: combined.trim().to_string(),
    })
}
