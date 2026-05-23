use crate::errors::{CpkgError, CpkgResult};
use crate::types::{Dependency, LockedPackage};
use semver::{Version, VersionReq};
use std::collections::HashMap;

pub fn resolve_dependencies(
    deps: &[Dependency],
    _existing_lock: Option<&HashMap<String, LockedPackage>>,
) -> CpkgResult<Vec<LockedPackage>> {
    let mut resolved: HashMap<String, LockedPackage> = HashMap::new();

    for dep in deps {
        resolve_one(dep, &mut resolved)?;
    }

    Ok(resolved.into_values().collect())
}

fn resolve_one(
    dep: &Dependency,
    resolved: &mut HashMap<String, LockedPackage>,
) -> CpkgResult<()> {
    if resolved.contains_key(&dep.name) {
        return Ok(());
    }

    let req = VersionReq::parse(&dep.version)
        .map_err(|e| CpkgError::VersionResolutionFailed(dep.name.clone(), e.to_string()))?;

    let candidates = query_registry(&dep.name)?;
    let best = candidates
        .iter()
        .filter(|v| req.matches(v))
        .max()
        .cloned()
        .ok_or_else(|| CpkgError::VersionResolutionFailed(dep.name.clone(), dep.version.clone()))?;

    let pkg = LockedPackage {
        name: dep.name.clone(),
        version: best,
        source: format!("registry+{}", dep.name),
        sha256: None,
        dependencies: vec![],
        include_dirs: vec!["include".into()],
        lib_files: vec![],
        lib_dir: None,
    };

    resolved.insert(dep.name.clone(), pkg);
    Ok(())
}

// Placeholder — will be replaced by actual HTTP registry calls
fn query_registry(name: &str) -> CpkgResult<Vec<Version>> {
    match name {
        "fmt" => Ok(vec![Version::new(9, 0, 0)]),
        "spdlog" => Ok(vec![Version::new(1, 14, 0)]),
        _ => Err(CpkgError::PackageNotFound(name.to_string())),
    }
}
