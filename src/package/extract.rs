use crate::errors::{CpkgError, CpkgResult};
use flate2::read::GzDecoder;
use std::fs;
use std::path::Path;
use tar::Archive;

pub fn extract_tarball(tarball: &Path, dest: &Path) -> CpkgResult<()> {
    log::info!("Extracting {} to {}", tarball.display(), dest.display());

    let file = fs::File::open(tarball)
        .map_err(|e| CpkgError::ExtractionError(e.to_string()))?;

    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    // Create a temp dir first, then move after successful extraction
    let tmp_dest = dest.with_extension("tmp");
    if tmp_dest.exists() {
        fs::remove_dir_all(&tmp_dest)?;
    }

    archive
        .unpack(&tmp_dest)
        .map_err(|e| CpkgError::ExtractionError(e.to_string()))?;

    // Handle tarballs that unpack into a single top-level directory
    // e.g., fmt-9.0.0.tar.gz → fmt-9.0.0/ → strip one level
    let entries: Vec<_> = fs::read_dir(&tmp_dest)?
        .filter_map(|e| e.ok())
        .collect();

    if entries.len() == 1 && entries[0].file_type()?.is_dir() {
        let inner = entries[0].path();
        if dest.exists() {
            fs::remove_dir_all(dest)?;
        }
        fs::rename(&inner, dest)?;
        fs::remove_dir_all(&tmp_dest)?;
    } else {
        // No single top-level dir — rename as-is
        if dest.exists() {
            fs::remove_dir_all(dest)?;
        }
        fs::rename(&tmp_dest, dest)?;
    }

    log::info!("Extracted to {}", dest.display());
    Ok(())
}
