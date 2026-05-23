pub mod detect;
pub mod version;

use crate::errors::CpkgResult;

pub fn detect_all() -> CpkgResult<Vec<crate::types::Compiler>> {
    detect::detect_all()
}

pub fn list() -> CpkgResult<()> {
    let compilers = detect_all()?;
    if compilers.is_empty() {
        println!("No C++ compilers detected.");
    } else {
        for c in &compilers {
            println!("{}  v{}  at {}", c.name.display(), c.version, c.path.display());
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub fn resolve(preference: Option<&str>) -> CpkgResult<crate::types::Compiler> {
    detect::resolve(preference)
}
