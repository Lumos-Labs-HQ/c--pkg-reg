use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum CpkgError {
    #[error("no supported C++ compiler found on PATH")]
    NoCompilerFound,

    #[error("failed to parse compiler version: {0}")]
    VersionParseError(String),

    #[error("compiler '{0}' not found")]
    CompilerNotFound(String),

    #[error("cpkg.toml not found in: {0}")]
    ConfigNotFound(String),

    #[error("failed to parse config ({0}): {1}")]
    ConfigParseError(String, String),

    #[error("failed to serialize {0}: {1}")]
    SerializeError(String, String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("cannot resolve dependency '{0}' with constraint '{1}'")]
    VersionResolutionFailed(String, String),

    #[error("download failed ({0}): {1}")]
    DownloadError(String, String),

    #[error("checksum mismatch: expected {0}, got {1}")]
    ChecksumMismatch(String, String),

    #[error("extraction failed: {0}")]
    ExtractionError(String),

    #[error("build failed for {0}: {1}")]
    BuildError(String, String),

    #[error("package '{0}' not found in registry")]
    PackageNotFound(String),

    #[error("script '{0}' not found in cpkg.toml")]
    ScriptNotFound(String),

    #[error("script '{0}' exited with code {1}")]
    ScriptFailed(String, i32),

    #[error("{0}")]
    General(String),
}

pub type CpkgResult<T> = Result<T, CpkgError>;
