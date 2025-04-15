use anyhow::Context;
use clap::{Parser, Subcommand};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use sha1::{Digest, Sha1};
use std::{
    ffi::CStr,
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
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
        pretty_print: bool,

        object_hash: String,
    },

    HashObject {
        #[clap(short = 'w')]
        write: bool,

        file: PathBuf,
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
                fs::create_dir(".git/objects").unwrap();
                fs::create_dir(".git/refs").unwrap();
                fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
                println!("Initialized empty Git repository in .git/");
            }
            Commands::CatFile {
                pretty_print,
                object_hash,
            } => {
                anyhow::ensure!(pretty_print, "mode should contain -p");
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
            Commands::HashObject { write, file } => {
                fn write_blob<W>(file: &Path, writer: W) -> anyhow::Result<String>
                where
                    W: Write,
                {
                    let stat = fs::metadata(&file)
                        .with_context(|| format!("file stat: {}", file.display()))?;

                    let writer = ZlibEncoder::new(writer, Compression::default());
                    let mut writer = HashWriter {
                        writer,
                        hasher: Sha1::new(),
                    };

                    write!(writer, "blob ")?;
                    write!(writer, "{}\0", stat.len())?;
                    let mut file = fs::File::open(&file)
                        .with_context(|| format!("open file: {}", &file.display()))?;
                    std::io::copy(&mut file, &mut writer).with_context(|| format!("copy file"))?;
                    let _ = writer.writer.finish()?;
                    let hash = writer.hasher.finalize();
                    Ok(hex::encode(hash))
                }
                let hash = if write {
                    let tmp = "temporary";
                    let hash = write_blob(
                        &file,
                        fs::File::create(tmp).context("temporary file for blob")?,
                    )
                    .context("write out blob ")?;
                    fs::create_dir_all(format!(".git/objects/{}", &hash[..2]))
                        .context("create subdir .git/objects/xx")?;
                    fs::rename(tmp, format!(".git/objects/{}/{}", &hash[..2], &hash[2..]))
                        .context("rename temporary file to .git/objects/xx/yy")?;
                    hash
                } else {
                    write_blob(&file, std::io::sink()).context("")?
                };

                println!("{hash}");
            }
        }
    } else {
        println!("No command provided. Use --help for more information.");
    }

    Ok(())
}

struct HashWriter<W> {
    writer: W,
    hasher: Sha1,
}

impl<W> Write for HashWriter<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.writer.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
