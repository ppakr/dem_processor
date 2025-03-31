use clap::Parser;
use std::path::PathBuf;

/// Convert ASC DEM files to grayscale or hillshaded PNGs
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    /// Input directory containing ASC files
    #[arg(short, long)]
    pub input_dir: PathBuf,

    /// Output directory for PNG files
    #[arg(short, long)]
    pub output_dir: PathBuf,

    /// Rendering mode: grayscale or hillshade
    #[arg(short, long, default_value = "grayscale")]
    pub mode: String,
}
