use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Compiler {
    pub name: CompilerName,
    pub path: PathBuf,
    pub version: CompilerVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompilerName {
    Gcc,
    Clang,
    Msvc,
    Unknown(String),
}

impl CompilerName {
    pub fn display(&self) -> &str {
        match self {
            CompilerName::Gcc => "g++ (GCC)",
            CompilerName::Clang => "clang++",
            CompilerName::Msvc => "MSVC (cl.exe)",
            CompilerName::Unknown(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompilerVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub raw: String,
}

impl std::fmt::Display for CompilerVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CppStandard {
    #[serde(rename = "c++11")]
    Cpp11,
    #[serde(rename = "c++14")]
    Cpp14,
    #[serde(rename = "c++17")]
    Cpp17,
    #[serde(rename = "c++20")]
    Cpp20,
    #[serde(rename = "c++23")]
    Cpp23,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpkgToml {
    pub package: PackageMeta,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub scripts: Vec<ScriptDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiler: Option<CompilerPreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpp_standard: Option<CppStandard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMeta {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerPreference {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub global: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptDef {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpkgLock {
    pub metadata: LockMetadata,
    pub packages: Vec<LockedPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockMetadata {
    #[serde(rename = "lockfile-version")]
    pub lockfile_version: u32,
    #[serde(rename = "generated-by")]
    pub generated_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    pub name: String,
    pub version: Version,
    pub source: String,
    pub sha256: Option<String>,
    pub dependencies: Vec<String>,
    pub include_dirs: Vec<String>,
    pub lib_files: Vec<String>,
    pub lib_dir: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum InstallTarget {
    Local(PathBuf),
    Global,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum LinkStrategy {
    Symlink,
    #[allow(dead_code)]
    Copy,
}
