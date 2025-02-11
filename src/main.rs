use std::error::Error;
use clap::Parser;
use fastsave::{Cli, run_script};

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let output_dir = run_script(&cli)?;
    println!("Fastsave completed. Output saved to: {}/fastsave.yaml", output_dir);
    Ok(())
}