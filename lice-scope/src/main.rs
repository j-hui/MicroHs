mod app;
mod gui;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::app::CombApp;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// View combinator graph in graphical UI
    Graph {
        #[arg(value_name = "input.comb")]
        /// Combinator file path
        filename: PathBuf,
    },
    // Other commands: strings, redexes, dot
}

fn main() {
    let args = Cli::parse();
    match args.cmd {
        Commands::Graph { filename } => CombApp::run(filename),
    }
}
