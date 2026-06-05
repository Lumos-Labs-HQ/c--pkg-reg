use crate::errors::{CpkgError, CpkgResult};
use crate::types::{Dependency, LockedPackage};
use semver::{Version, VersionReq};
use std::collections::HashMap;

const VCPKG_CATALOG_URL: &str = "https://vcpkg.io/output.json";

pub fn resolve_dependencies(
    deps: &[Dependency],
    _existing_lock: Option<&HashMap<String, LockedPackage>>,
) -> CpkgResult<Vec<LockedPackage>> {
    // Load catalog once for the whole batch
    let catalog = load_vcpkg_catalog();

    let mut resolved: HashMap<String, LockedPackage> = HashMap::new();
    for dep in deps {
        resolve_one(dep, &mut resolved, &catalog)?;
    }
    Ok(resolved.into_values().collect())
}

fn resolve_one(
    dep: &Dependency,
    resolved: &mut HashMap<String, LockedPackage>,
    catalog: &HashMap<String, String>, // name → github "owner/repo"
) -> CpkgResult<()> {
    if resolved.contains_key(&dep.name) {
        return Ok(());
    }

    let req = VersionReq::parse(&dep.version)
        .map_err(|e| CpkgError::VersionResolutionFailed(dep.name.clone(), e.to_string()))?;

    let (owner, repo) = if dep.name.contains('/') {
        // Explicit "owner/repo" syntax — use as-is
        let slash = dep.name.find('/').unwrap();
        (dep.name[..slash].to_string(), dep.name[slash + 1..].to_string())
    } else {
        // Look up in vcpkg catalog first, then fall back to GitHub search
        match catalog.get(&dep.name.to_lowercase()) {
            Some(slug) => {
                let slash = slug.find('/').unwrap();
                log::debug!("vcpkg catalog: {} -> {}", dep.name, slug);
                (slug[..slash].to_string(), slug[slash + 1..].to_string())
            }
            None => {
                log::info!("'{}' not in vcpkg catalog, searching GitHub...", dep.name);
                search_github_repo(&dep.name).ok_or_else(|| {
                    CpkgError::PackageNotFound(format!(
                        "could not find '{}'. Try 'owner/repo' syntax, e.g. 'ibireme/yyjson'",
                        dep.name
                    ))
                })?
            }
        }
    };

    let candidates = query_github_tags(&owner, &repo)?;
    let best = candidates
        .iter()
        .filter(|v| req.matches(v))
        .max()
        .cloned()
        .ok_or_else(|| CpkgError::VersionResolutionFailed(dep.name.clone(), dep.version.clone()))?;

    resolved.insert(dep.name.clone(), LockedPackage {
        name: dep.name.clone(),
        version: best,
        source: format!("github.com/{}/{}", owner, repo),
        sha256: None,
        dependencies: vec![],
        include_dirs: vec!["include".into()],
        lib_files: vec![],
        lib_dir: None,
    });
    Ok(())
}

/// Download and parse the vcpkg catalog, returning a map of lowercase package name → "owner/repo".
/// Cached to .cpkg/cache/vcpkg-catalog.json for 24 hours.
fn load_vcpkg_catalog() -> HashMap<String, String> {
    let cache_path = cache_path();

    // Use cached file if it exists and is < 24 hours old
    if let Some(path) = &cache_path {
        if let Ok(meta) = std::fs::metadata(path) {
            if let Ok(modified) = meta.modified() {
                if modified.elapsed().map(|d| d.as_secs()).unwrap_or(u64::MAX) < 86400 {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        if let Ok(map) = serde_json::from_str(&content) {
                            log::debug!("Using cached vcpkg catalog");
                            return map;
                        }
                    }
                }
            }
        }
    }

    log::info!("Fetching vcpkg catalog from {}...", VCPKG_CATALOG_URL);
    let map = fetch_vcpkg_catalog().unwrap_or_else(|e| {
        log::warn!("Failed to fetch vcpkg catalog: {}. Falling back to GitHub search.", e);
        HashMap::new()
    });

    // Save to cache
    if let Some(path) = &cache_path {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string(&map) {
            let _ = std::fs::write(path, json);
        }
    }

    map
}

fn fetch_vcpkg_catalog() -> CpkgResult<HashMap<String, String>> {
    let response = github_client()
        .get(VCPKG_CATALOG_URL)
        .send()
        .map_err(|e| CpkgError::DownloadError(VCPKG_CATALOG_URL.into(), e.to_string()))?;

    if !response.status().is_success() {
        return Err(CpkgError::DownloadError(
            VCPKG_CATALOG_URL.into(),
            format!("HTTP {}", response.status()),
        ));
    }

    let data: serde_json::Value = response
        .json()
        .map_err(|e| CpkgError::ConfigParseError("vcpkg catalog".into(), e.to_string()))?;

    let mut map = HashMap::new();
    if let Some(sources) = data["Source"].as_array() {
        for pkg in sources {
            let name = pkg["Name"].as_str().unwrap_or("").to_lowercase();
            let homepage = pkg["homepage"].as_str().unwrap_or("");
            if let Some(slug) = github_slug_from_url(homepage) {
                map.insert(name, slug);
            }
        }
    }

    log::info!("vcpkg catalog loaded: {} packages", map.len());
    Ok(map)
}

/// Extract "owner/repo" from a GitHub URL like "https://github.com/fmtlib/fmt"
fn github_slug_from_url(url: &str) -> Option<String> {
    let path = url.strip_prefix("https://github.com/")?;
    // Must have exactly one slash and non-empty parts
    let mut parts = path.splitn(2, '/');
    let owner = parts.next().filter(|s| !s.is_empty())?;
    let repo = parts.next().filter(|s| !s.is_empty())?;
    // Strip any trailing path (e.g. /tree/main)
    let repo = repo.split('/').next()?;
    Some(format!("{}/{}", owner, repo))
}

fn cache_path() -> Option<std::path::PathBuf> {
    // Try local .cpkg/cache first, fall back to global cache dir
    let local = std::path::PathBuf::from(".cpkg/cache/vcpkg-catalog.json");
    if local.parent().map(|p| p.exists()).unwrap_or(false) {
        return Some(local);
    }
    dirs::cache_dir().map(|d| d.join("cpkg").join("vcpkg-catalog.json"))
}

fn query_github_tags(owner: &str, repo: &str) -> CpkgResult<Vec<Version>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/tags?per_page=100",
        owner, repo
    );
    log::debug!("Querying GitHub tags: {}", url);

    let response = github_client()
        .get(&url)
        .send()
        .map_err(|e| CpkgError::PackageNotFound(format!("{}: {}", repo, e)))?;

    if !response.status().is_success() {
        return Err(CpkgError::PackageNotFound(format!(
            "{} (HTTP {})", repo, response.status()
        )));
    }

    let tags: Vec<serde_json::Value> = response
        .json()
        .map_err(|e| CpkgError::PackageNotFound(e.to_string()))?;

    let versions: Vec<Version> = tags.iter()
        .filter_map(|t| {
            let name = t["name"].as_str()?;
            let stripped = name.strip_prefix('v').unwrap_or(name);
            Version::parse(stripped).ok()
        })
        .collect();

    if versions.is_empty() {
        return Err(CpkgError::PackageNotFound(format!(
            "no valid semver tags found for {}/{}", owner, repo
        )));
    }
    Ok(versions)
}

fn search_github_repo(name: &str) -> Option<(String, String)> {
    let url = format!(
        "https://api.github.com/search/repositories?q={}+in:name+fork:true&sort=stars&order=desc&per_page=10",
        name
    );

    let response = github_client().get(&url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }

    #[derive(serde::Deserialize)]
    struct SearchResult { items: Vec<RepoItem> }
    #[derive(serde::Deserialize)]
    struct RepoItem { full_name: String, language: Option<String> }

    let results: SearchResult = response.json().ok()?;
    let best = results.items.iter()
        .find(|i| matches!(i.language.as_deref(), Some("C") | Some("C++")))
        .or_else(|| results.items.first())?;

    let mut parts = best.full_name.splitn(2, '/');
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.to_string();
    log::info!("GitHub search found: {}/{}", owner, repo);
    Some((owner, repo))
}

fn github_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("cpkg/0.2.0")
        .build()
        .expect("failed to build HTTP client")
}
