use clap::Parser;
use log::{error, info};
use regex::Regex;
use simple_logger::SimpleLogger;
use std::net::Ipv4Addr;
use std::path::Path;
use std::time;
use zx0::{CompressionResult, Compressor};
use indicatif::ProgressBar;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio::net::TcpStream;

const CARGO_PKG_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

/// Utility to send arbitrary files to a WiFi equipped ZX Spectrum
#[derive(Debug, Parser)]
#[command(about)]
pub struct Arguments {
    /// IP address of ZX Spectrum that's runs zxmit
    pub ip: Ipv4Addr,
    /// File name of filename to deliver
    pub filename: String,
    /// Dummy run without any networking communication
    #[arg(short, long)]
    pub dummy: bool,
    /// Don't use compression
    #[arg(short, long)]
    pub no_compression: bool,
}

const LONG_HEADER_LEN: usize = 17;
const CHUNK_SIZE: usize = 1024;
const FLAGS_COMPRESSED: u8 = 1;
const FLAGS_LONG_HEADER: u8 = 2;

async fn transmit(
    ip: Ipv4Addr,
    name: Vec<u8>,
    buffer: Vec<u8>,
    dummy: bool,
    no_compression: bool,
) -> std::io::Result<()> {
    let addr = format!("{}:6144", ip);

    info!("Establishing connection to {}", &addr);
    let mut stream = if dummy {
        None
    } else {
        Some(TcpStream::connect(addr).await?)
    };
    let mut compressed_bytes = 0;
    let blocks_num = buffer.chunks(CHUNK_SIZE).len();
    let total_bytes = buffer.len();

    let now = time::Instant::now();

    let (tx, mut rx) = mpsc::channel(16);
    tokio::spawn(async move {
        let mut seq: u8 = 0;
        let mut long_header = true;
        for chunk in buffer.chunks(CHUNK_SIZE) {
            let mut block: Vec<u8>;
            let compressed = if no_compression {
                CompressionResult {
                    output: Vec::new(),
                    delta: 0,
                }
            } else {
                Compressor::new().quick_mode(true).compress(chunk)
            };

            let use_compressed: bool = !no_compression
                && chunk.len() == CHUNK_SIZE
                && compressed.output.len() < chunk.len();

            let mut to_send = if !use_compressed {
                Vec::from(chunk)
            } else {
                compressed.output
            };

            let mut flags: u8 = 0;

            if use_compressed {
                flags |= FLAGS_COMPRESSED;
            }

            if long_header {
                flags |= FLAGS_LONG_HEADER;
            }

            block = vec![
                seq,
                (to_send.len() % 256) as u8,
                (to_send.len() / 256) as u8,
                flags,
            ];

            seq = seq.wrapping_add(1);

            if long_header {
                block.append(&mut name.clone());
                block.resize(LONG_HEADER_LEN, 0);
                long_header = false;
            }
            block.append(&mut to_send);
            tx.send(block).await.unwrap();
        }
    });

    let bar = ProgressBar::new(blocks_num as u64);
    while let Some(block) = rx.recv().await {
        let seq = block[0];
        if let Some(ref mut s) = stream {
            s.write_all(&block).await?
        };

        bar.inc(1);
        compressed_bytes += block.len();
        let mut acked = 0;
        loop {
            // ACK is:
            // 0: sequence
            // 1: error code, 0 if OK, 1 otherwise
            // 2, 3: acked size (includes header), LE
            let mut read_buf = [0u8; 4];
            if let Some(ref mut stream) = stream {
                stream.read_exact(&mut read_buf).await?
            } else {
                break;
            };
            if read_buf[0] != seq {
                info!("Got out of order ACK: {} instead of {}", read_buf[0], seq);
                continue;
            }
            acked += read_buf[2] as usize + read_buf[3] as usize * 256;
            if acked == block.len() {
                break;
            }
        }
    }
    bar.finish();

    info!(
        "Compressed {} bytes into {} bytes, ratio: {}, elapsed: {:.2?}",
        total_bytes,
        compressed_bytes,
        compressed_bytes as f32 / total_bytes as f32,
        now.elapsed()
    );
    Ok(())
}

fn split_at_last_dot(filename: &str) -> (String, String) {
    let parts: Vec<&str> = filename.split('.').collect();

    if parts.len() <= 1 {
        return (filename.to_string(), "".to_string());
    }

    let last = parts.last().unwrap().to_string();
    let first = parts[..parts.len() - 1].join(" ");

    (first, last)
}

fn filename_to_short(filename: &str) -> String {
    let (mut name, mut extension) = split_at_last_dot(filename);

    if name.len() > 8 {
        name = name.chars().take(8).collect();
    }
    if extension.len() > 3 {
        extension = extension.chars().take(3).collect();
    }

    let re = Regex::new(r"[ \t\.\\/]").unwrap();

    name = re.replace_all(&name, "_").to_string();
    extension = re.replace_all(&extension, "_").to_string();

    std::format!("{}.{}", name, extension)
}

async fn process(args: Arguments) -> std::io::Result<()> {
    let file = tokio::fs::read(args.filename.clone()).await?;
    let filename = args.filename.clone();
    let path = Path::new(&filename);
    let basename = String::from(path.file_name().unwrap().to_str().unwrap());
    let namebuf: Vec<u8> = filename_to_short(&basename).into();

    info!(
        "Short filaname will be {}",
        String::from_utf8(namebuf.clone()).unwrap()
    );
    assert!(namebuf.len() <= 12);

    transmit(args.ip, namebuf, file, args.dummy, args.no_compression).await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    println!(
        "zxmit {} (c) Alex Nihirash & Vasily Khoruzhick",
        CARGO_PKG_VERSION.unwrap_or("dev")
    );
    let args = Arguments::parse();

    SimpleLogger::new().init().unwrap();

    match process(args).await {
        Err(e) => error!("{}", e.to_string()),
        _ => info!("Done!"),
    }
}
