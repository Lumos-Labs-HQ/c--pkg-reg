use crate::config::project;
use crate::errors::{CpkgError, CpkgResult};

pub fn run(script_name: &str, extra_args: &[String]) -> CpkgResult<()> {
    let config_path = project::find_config_path(".")?;
    let config = project::load_config(&config_path)?;

    let script = config
        .scripts
        .iter()
        .find(|s| s.name == script_name)
        .ok_or_else(|| CpkgError::ScriptNotFound(script_name.to_string()))?;

    println!("> {}", script.command);

    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c").arg(&script.command);
    for arg in extra_args {
        cmd.arg(arg);
    }

    let status = cmd.status()?;

    if !status.success() {
        return Err(CpkgError::ScriptFailed(
            script_name.to_string(),
            status.code().unwrap_or(1),
        ));
    }

    Ok(())
}
