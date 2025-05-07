use clap::Parser;
use log::{error, info};
use simple_logger::SimpleLogger;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::path::Path;
use regex::Regex;

const CARGO_PKG_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

/// This utility used for delivery filename to ZX Spectrum running zxmit
#[derive(Debug, Parser)]
#[command(about)]
pub struct Arguments {
    /// IP address of ZX Spectrum that's runs zxmit
    pub ip: Ipv4Addr,
    /// File name of filename to deliver
    pub filename: String,
}

fn read_file(name: String) -> std::io::Result<Vec<u8>> {
    info!("Reading file '{}' ", name);
    let mut file = File::open(name)?;
    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

const HEADER_LEN: usize = 17;

fn transmit(ip: Ipv4Addr, name: Vec<u8>, buffer: Vec<u8>) -> std::io::Result<()> {
    let addr = format!("{}:6144", ip);

    info!("Establishing connection to {}", &addr);
    let mut stream = TcpStream::connect(addr)?;
    let mut seq: u8 = 0;

    let total = buffer.len();
    let mut written = 0;

    for chunk in buffer.chunks(1024) {
        let mut block: Vec<u8>;

        block = vec![
                        seq,
                        (chunk.len() % 256) as u8,
                        (chunk.len() / 256) as u8,
                        0 // no compression
        ];

        block.append(&mut name.clone());
        block.resize(HEADER_LEN, 0);
        block.append(&mut Vec::from(chunk));

        stream.write_all(&block).unwrap();
        written += chunk.len();

        info!("Sent {} out of {}", written, total);
        let mut acked = 0;
        loop {
            // ACK is:
            // 0: sequence
            // 1: error code, 0 if OK, 1 otherwise
            // 2, 3: acked size (includes header), LE
            let mut read_buf = [0u8; 4];
            stream.read_exact(&mut read_buf).unwrap();
            if read_buf[0] != seq {
                info!("Got out of order ACK: {} instead of {}", read_buf[0], seq);
                continue;
            }
            acked += read_buf[2] as usize + read_buf[3] as usize * 256;
            if acked == block.len() {
                break;
            }
        }

        seq = seq.wrapping_add(1);
    }

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

fn process(args: Arguments) -> std::io::Result<()> {
    let file = read_file(args.filename.clone())?;
    let filename = args.filename.clone();
    let path = Path::new(&filename);
    let basename = String::from(path.file_name().unwrap().to_str().unwrap());
    let namebuf: Vec<u8> = filename_to_short(&basename).into();

    info!("Short filaname will be {}", String::from_utf8(namebuf.clone()).unwrap());
    assert!(namebuf.len() <= 12);

    transmit(args.ip, namebuf, file)?;

    Ok(())
}

fn main() {
    println!(
        "zxmit {} (c) Alex Nihirash & Vasily Khoruzhick",
        CARGO_PKG_VERSION.unwrap_or("dev")
    );
    let args = Arguments::parse();

    SimpleLogger::new().init().unwrap();

    match process(args) {
        Err(e) => error!("{}", e.to_string()),
        _ => info!("Done!"),
    }
}
