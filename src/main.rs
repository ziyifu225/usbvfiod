mod cli;

use clap::Parser;
use cli::Cli;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _args = Cli::parse();

    Ok(())
}
