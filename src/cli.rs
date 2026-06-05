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
    Init {
        #[arg(default_value = ".")]
        path: String,
    },
    Compilers,
    Install {
        package: String,
        #[arg(long)]
        global: bool,
        #[arg(long)]
        version: Option<String>,
    },
    Remove {
        package: String,
    },
    Run {
        script: String,
        #[arg(last = true)]
        args: Vec<String>,
    },
    Build,
}
