mod cli;
mod processor;

use clap::Parser;
use walkdir::WalkDir;
use std::fs;
use crate::cli::Args;
use crate::processor::process_asc_file;

fn main()-> anyhow::Result<()>{

    println!("Starting DEM Processor...");
    
    let args = Args::parse();

    if !args.output_dir.exists() {
        fs::create_dir_all(&args.output_dir)?;
    }

    for entry in WalkDir::new(&args.input_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().map(|ext| ext == "asc").unwrap_or(false))
    {
        let path = entry.path();
        println!("Processing: {:?}", path);
        process_asc_file(path, &args)?;
    }

    Ok(())
}
