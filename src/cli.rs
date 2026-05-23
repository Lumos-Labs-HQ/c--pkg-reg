use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cpkg", about = "A C++ package manager", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Scaffold a new cpkg.toml
    Init {
        #[arg(default_value = ".")]
        path: String,
    },
    /// Detect and list available C++ compilers
    Compilers,
    /// Install a dependency
    Install {
        package: String,
        #[arg(long)]
        global: bool,
        #[arg(long)]
        version: Option<String>,
    },
    /// Remove a dependency
    Remove {
        package: String,
    },
    /// Run a named script from cpkg.toml
    Run {
        script: String,
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Run the default "build" script
    Build,
}
