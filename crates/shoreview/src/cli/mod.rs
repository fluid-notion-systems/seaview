use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "shoreview")]
#[command(about = "A 3D mesh viewer for STL files", long_about = None)]
pub struct Args {
    /// Path to an STL file or directory containing a sequence
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

impl Args {
    pub fn parse_args() -> Self {
        Args::parse()
    }
}
