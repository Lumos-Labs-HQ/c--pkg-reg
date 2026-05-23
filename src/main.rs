mod cli;
mod compiler;
mod config;
mod errors;
mod init;
mod package;
mod paths;
mod script;
mod types;

use clap::Parser;
use errors::CpkgResult;

fn main() -> CpkgResult<()> {
    let args = cli::Cli::parse();

    if args.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::init();
    }

    match args.command {
        cli::Command::Init { path } => init::run(&path),
        cli::Command::Compilers => compiler::list(),
        cli::Command::Install {
            package,
            global,
            version,
        } => package::install(&package, global, version.as_deref()),
        cli::Command::Remove { package } => package::uninstall(&package),
        cli::Command::Run { script, args } => script::run(&script, &args),
        cli::Command::Build => script::run("build", &[]),
    }
}
