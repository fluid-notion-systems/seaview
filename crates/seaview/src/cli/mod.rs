use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "seaview")]
#[command(about = "A 3D mesh viewer for STL files", long_about = None)]
pub struct Args {
    /// Path to an STL file or directory containing a sequence
    pub path: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Source coordinate system (yup, zup, fluidx3d)
    #[arg(long, default_value = "yup")]
    pub source_coordinates: String,
}

impl Args {
    pub fn parse_args() -> Self {
        Args::parse()
    }
}
