use anyhow::Context;
use clap::{Parser, Subcommand};
use flate2::read::ZlibDecoder;
use std::{
    ffi::CStr,
    fs,
    io::{BufRead, BufReader, Read},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Init,

    CatFile {
        #[clap(short = 'p')]
        preety_print: bool,

        object_hash: String,
    },
}

#[derive(Debug)]
enum Kind {
    Blob,
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
            Commands::CatFile {
                preety_print,
                object_hash,
            } => {
                anyhow::ensure!(preety_print, "mode should contain -p");
                let path = format!(".git/objects/{}/{}", &object_hash[..2], &object_hash[2..]);
                let file = fs::File::open(&path).context("open in .git/objects")?;

                // Convert the file to a ZlibDecoder
                let z = ZlibDecoder::new(file);
                let mut z = BufReader::new(z);
                let mut buf = Vec::new();
                // Read the header from the ZlibDecoder
                // The header is expected to be in the format "kind size\0content"
                z.read_until(0, &mut buf)
                    .context("read header from .git/objects")?;
                let header = CStr::from_bytes_with_nul(&buf)
                    .context("know there is exactly one nul, at the end")?;
                let header = header
                    .to_str()
                    .context(".git/objects file header is not utf-8 ? wtf")?;
                let Some((kind, size)) = header.split_once(" ") else {
                    anyhow::bail!("{path} file header did not start with a know type: {header}");
                };
                let kind = match kind {
                    "blob" => Kind::Blob,
                    _ => {
                        anyhow::bail!("to be impl for kind: {kind}")
                    }
                };
                let size = size
                    .parse::<u64>()
                    .context("parse size from .git/objects header size: {size}")?;
                let mut z = z.take(size);
                match kind {
                    Kind::Blob => {
                        let stdout = std::io::stdout();
                        let mut stdout = stdout.lock();
                        let n = std::io::copy(&mut z, &mut stdout)
                            .context("write .git/object to stdout")?;
                        anyhow::ensure!(
                            n == size,
                            ".git/objects file was not the expected size: (expected: {size}, actual: {n})"
                        )
                    }
                }
            }
        }
    } else {
        println!("No command provided. Use --help for more information.");
    }

    Ok(())
}
