use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "shoreview")]
#[command(about = "A 3D mesh viewer for STL files", long_about = None)]
pub struct Args {
    /// Path to the STL file to load
    #[arg(value_name = "FILE")]
    pub stl_file: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

impl Args {
    pub fn parse_args() -> Self {
        Args::parse()
    }
}
