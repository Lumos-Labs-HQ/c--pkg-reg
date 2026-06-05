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

    // Resolve owner/repo: explicit "owner/repo" → parse directly;
    // plain name → priority registry first, then GitHub search fallback
    let (owner, repo, candidates) = if dep.name.contains('/') {
        let slash = dep.name.find('/').unwrap();
        let o = &dep.name[..slash];
        let r = &dep.name[slash + 1..];
        let tags = query_github_tags(o, r)?;
        (o.to_string(), r.to_string(), tags)
    } else {
        match priority_repo(&dep.name) {
            Some((o, r)) => {
                log::debug!("Registry hit: {}/{}", o, r);
                let tags = query_github_tags(&o, &r)?;
                (o, r, tags)
            }
            None => match search_github_repo(&dep.name) {
                Some((o, r)) => {
                    log::info!("Auto-discovered: {}/{}", o, r);
                    let tags = query_github_tags(&o, &r)?;
                    (o, r, tags)
                }
                None => return Err(CpkgError::PackageNotFound(format!(
                    "could not find '{}' on GitHub. Try 'owner/repo' syntax, e.g. 'ibireme/yyjson'",
                    dep.name
                ))),
            },
        }
    };
    let best = candidates
        .iter()
        .filter(|v| req.matches(v))
        .max()
        .cloned()
        .ok_or_else(|| CpkgError::VersionResolutionFailed(dep.name.clone(), dep.version.clone()))?;

    let source = format!("github.com/{}/{}", owner, repo);

    let pkg = LockedPackage {
        name: dep.name.clone(),
        version: best,
        source,
        sha256: None,
        dependencies: vec![],
        include_dirs: vec!["include".into()],
        lib_files: vec![],
        lib_dir: None,
    };

    resolved.insert(dep.name.clone(), pkg);
    Ok(())
}

fn query_github_tags(owner: &str, repo: &str) -> CpkgResult<Vec<Version>> {
    use semver::Version;

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
            "{} (HTTP {})",
            repo,
            response.status()
        )));
    }

    let tags: Vec<serde_json::Value> = response
        .json()
        .map_err(|e| CpkgError::PackageNotFound(e.to_string()))?;

    let mut versions: Vec<Version> = Vec::new();
    for tag in &tags {
        let name = tag["name"].as_str().unwrap_or("");
        let stripped = name.strip_prefix('v').unwrap_or(name);
        if let Ok(v) = Version::parse(stripped) {
            versions.push(v);
        }
    }

    if versions.is_empty() {
        return Err(CpkgError::PackageNotFound(format!(
            "no valid semver tags found for {}/{}",
            owner, repo
        )));
    }

    Ok(versions)
}

fn search_github_repo(name: &str) -> Option<(String, String)> {
    log::info!("Searching GitHub for '{}' ...", name);

    // Priority registry — exact matches for packages that GitHub search gets wrong
    if let Some(r) = priority_repo(name) {
        return Some(r);
    }

    // GitHub search — look for repos with matching name, prioritize C/C++, sorted by stars
    let url = format!(
        "https://api.github.com/search/repositories?q={}+in:name+fork:true&sort=stars&order=desc&per_page=10",
        name
    );

    let response = github_client().get(&url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }

    #[derive(serde::Deserialize)]
    struct SearchResult {
        items: Vec<RepoItem>,
    }
    #[derive(serde::Deserialize)]
    struct RepoItem {
        full_name: String,
        language: Option<String>,
    }

    let results: SearchResult = response.json().ok()?;

    // Prefer repos with C/C++ language, then fall back to any
    let best = results.items.iter()
        .find(|i| matches!(i.language.as_deref(), Some("C") | Some("C++")))
        .or_else(|| results.items.first());

    let item = best?;
    let parts: Vec<&str> = item.full_name.splitn(2, '/').collect();
    if parts.len() == 2 {
        log::info!("Found: {}", item.full_name);
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

fn priority_repo(name: &str) -> Option<(String, String)> {
    let lower = name.to_lowercase();
    let (owner, repo) = match lower.as_str() {
        // Formatting / logging
        "fmt" | "libfmt"            => ("fmtlib",         "fmt"),
        "spdlog"                    => ("gabime",          "spdlog"),
        "glog"                      => ("google",          "glog"),
        "plog"                      => ("SergiusTheBest",  "plog"),
        // JSON / serialization
        "nlohmann_json" | "json"    => ("nlohmann",        "json"),
        "rapidjson"                 => ("Tencent",         "rapidjson"),
        "simdjson"                  => ("simdjson",        "simdjson"),
        "yyjson"                    => ("ibireme",         "yyjson"),
        "glaze"                     => ("stephenberry",    "glaze"),
        // HTTP / networking
        "httplib" | "cpp-httplib"   => ("yhirose",         "cpp-httplib"),
        "curl" | "libcurl"          => ("curl",            "curl"),
        "beast" | "boost.beast"     => ("boostorg",        "beast"),
        "cpr"                       => ("libcpr",          "cpr"),
        "restinio"                  => ("Stiffstream",     "restinio"),
        // Testing
        "gtest" | "googletest"      => ("google",          "googletest"),
        "catch2"                    => ("catchorg",        "Catch2"),
        "doctest"                   => ("doctest",         "doctest"),
        "boost.ut" | "ut"           => ("boost-ext",       "ut"),
        // Math / linear algebra
        "eigen" | "eigen3"          => ("libeigen",        "eigen"),
        "glm"                       => ("g-truc",          "glm"),
        "armadillo"                 => ("conradsnicta",    "armadillo-code"),
        "blaze"                     => ("parsa-epfl",      "blaze"),
        // Argument parsing / CLI
        "cli11"                     => ("CLIUtils",        "CLI11"),
        "argparse"                  => ("p-ranav",         "argparse"),
        "cxxopts"                   => ("jarro2783",       "cxxopts"),
        "clipp"                     => ("muellan",         "clipp"),
        // Parsing
        "re2"                       => ("google",          "re2"),
        "pcre2"                     => ("PCRE2Project",    "pcre2"),
        "antlr4"                    => ("antlr",           "antlr4"),
        // Compression
        "zlib"                      => ("madler",          "zlib"),
        "zstd"                      => ("facebook",        "zstd"),
        "lz4"                       => ("lz4",             "lz4"),
        "brotli"                    => ("google",          "brotli"),
        // Crypto / hashing
        "openssl"                   => ("openssl",         "openssl"),
        "mbedtls"                   => ("Mbed-TLS",        "mbedtls"),
        "xxhash"                    => ("Cyan4973",        "xxHash"),
        "blake3"                    => ("BLAKE3-team",     "BLAKE3"),
        // Data structures / utilities
        "absl" | "abseil"           => ("abseil",          "abseil-cpp"),
        "folly"                     => ("facebook",        "folly"),
        "boost"                     => ("boostorg",        "boost"),
        "range-v3"                  => ("ericniebler",     "range-v3"),
        "tl-expected" | "expected"  => ("TartanLlama",     "expected"),
        "outcome"                   => ("ned14",           "outcome"),
        "magic_enum"                => ("Neargye",         "magic_enum"),
        "nameof"                    => ("Neargye",         "nameof"),
        "frozen"                    => ("serge-sans-paille","frozen"),
        "entt"                      => ("skypjack",        "entt"),
        // Concurrency
        "taskflow"                  => ("taskflow",        "taskflow"),
        "tbb" | "onetbb"            => ("oneapi-src",      "oneTBB"),
        "liburing"                  => ("axboe",           "liburing"),
        // Config / data formats
        "toml11"                    => ("ToruNiina",       "toml11"),
        "tomlplusplus" | "toml++"  => ("marzer",          "tomlplusplus"),
        "yaml-cpp"                  => ("jbeder",          "yaml-cpp"),
        "pugixml"                   => ("zeux",            "pugixml"),
        "tinyxml2"                  => ("leethomason",     "tinyxml2"),
        // Image / graphics
        "stb"                       => ("nothings",        "stb"),
        "imgui"                     => ("ocornut",         "imgui"),
        "glfw"                      => ("glfw",            "glfw"),
        "glad"                      => ("Dav1dde",         "glad"),
        "sdl" | "sdl2"              => ("libsdl-org",      "SDL"),
        "vulkan-headers"            => ("KhronosGroup",    "Vulkan-Headers"),
        // Misc
        "cereal"                    => ("USCiLab",         "cereal"),
        "msgpack"                   => ("msgpack",         "msgpack-c"),
        "flatbuffers"               => ("google",          "flatbuffers"),
        "protobuf"                  => ("protocolbuffers", "protobuf"),
        "llhttp"                    => ("nodejs",          "llhttp"),
        "libuv"                     => ("libuv",           "libuv"),
        "sqlite3" | "sqlite"        => ("sqlite",          "sqlite"),
        "ryu"                       => ("ulfjack",         "ryu"),
        _ => return None,
    };
    Some((owner.into(), repo.into()))
}

fn github_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("cpkg/0.1.0")
        .build()
        .expect("failed to build HTTP client")
}
