use clap::{Parser, Subcommand};
use std::fs;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Init,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if let Some(command) = args.command {
        match command {
            Commands::Init => {
                fs::create_dir(".git").unwrap();
                fs::create_dir(".git/object").unwrap();
                fs::create_dir(".git/refs").unwrap();
                fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
                println!("Initialized empty Git repository in .git/");
            }
        }
    } else {
        println!("No command provided. Use --help for more information.");
    }

    Ok(())
}
