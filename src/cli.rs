use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
pub struct LowboyArgs {
    /// Load configuration from a custom location. Defaults to: $XDG_CONFIG/lowboy/config.yml
    #[arg(short, long = "config", value_name = "FILE")]
    pub config_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    /// Print a config template
    ConfigTemplate,
    /// Create a config file. Defaults to: $XDG_CONFIG/lowboy/config.yml
    ConfigInit {
        /// Create configuration at a custom location.
        #[arg(short, long = "config", value_name = "FILE")]
        config_path: Option<PathBuf>,
    },
}

#[derive(Clone, Debug, Parser)]
#[command(subcommand_negates_reqs(true))]
#[command(args_conflicts_with_subcommands(true))]
pub struct Cli {
    #[command(flatten)]
    pub args: LowboyArgs,

    #[command(subcommand)]
    pub command: Option<Command>,
}
