use std::path::PathBuf;
use regex::Regex;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use zx0::{CompressionResult, Compressor};
use iced::task::{Straw, sipper};

#[derive(Debug, Clone)]
pub enum UploadError {
    File,
    Connection,
}

#[derive(Debug, Clone)]
pub struct UploadProgress {
    pub current_block: usize,
    pub blocks_num: usize,
    pub total_bytes: usize,
    pub compressed_bytes: usize,
}

#[derive(Debug)]
pub struct FileUploader {
    pub address: String,
    pub filepath: PathBuf,
    pub use_compression: bool,
    pub dummy: bool,
}

const LONG_HEADER_LEN: usize = 17;
const CHUNK_SIZE: usize = 1024;
const FLAGS_COMPRESSED: u8 = 1;
const FLAGS_LONG_HEADER: u8 = 2;

impl FileUploader {
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
        let (mut name, mut extension) = Self::split_at_last_dot(filename);

        if name.len() > 8 {
            name = name.chars().take(8).collect();
        }
        if extension.len() > 3 {
            extension = extension.chars().take(3).collect();
        }

        let re = Regex::new(r"[ \t\.\\/]").unwrap();

        name = re.replace_all(&name, "_").to_string();
        extension = re.replace_all(&extension, "_").to_string();

        let res = std::format!("{}.{}", name, extension);

        res
    }

    pub fn upload(self) -> impl Straw<(), UploadProgress, UploadError> {
        sipper(async move |mut progress| {
            let buffer = tokio::fs::read(self.filepath.clone())
                .await
                .map_err(|_| UploadError::File)?;
            let basename = self.filepath.as_path().file_name().unwrap().to_str().unwrap();
            let name: Vec<u8> = Self::filename_to_short(basename).into();
            assert!(name.len() <= 12);

            let addr = format!("{}:6144", self.address);

            let mut stream = if !self.dummy {
                Some(TcpStream::connect(addr)
                    .await
                    .map_err(|_| UploadError::Connection)?)
            } else {
                None
            };

            let mut compressed_bytes = 0;
            let blocks_num = buffer.chunks(CHUNK_SIZE).len();
            let mut current_block = 1;
            let total_bytes = buffer.len();
            let (tx, mut rx) = mpsc::channel(16);
            let use_compression = self.use_compression;
            tokio::spawn(async move {
                let mut seq: u8 = 0;
                let mut long_header = true;
                for chunk in buffer.chunks(CHUNK_SIZE) {
                    let mut block: Vec<u8>;
                    let compressed = if !use_compression {
                        CompressionResult {
                            output: Vec::new(),
                            delta: 0,
                        }
                    } else {
                        Compressor::new().quick_mode(true).compress(chunk)
                    };

                    let use_compressed: bool = use_compression
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

            while let Some(block) = rx.recv().await {
                let seq = block[0];
                if let Some(ref mut s) = stream {
                    s.write_all(&block)
                        .await
                        .map_err(|_| UploadError::Connection)?;
                };

                compressed_bytes += block.len();
                current_block += 1;
                let _ = progress.send(UploadProgress {
                    current_block,
                    blocks_num,
                    total_bytes,
                    compressed_bytes,
                }).await;

                let mut acked = 0;
                loop {
                    // ACK is:
                    // 0: sequence
                    // 1: error code, 0 if OK, 1 otherwise
                    // 2, 3: acked size (includes header), LE
                    let mut read_buf = [0u8; 4];
                    if let Some(ref mut stream) = stream {
                        stream.read_exact(&mut read_buf)
                            .await
                            .map_err(|_| UploadError::Connection)?;
                    } else {
                        break;
                    };
                    if read_buf[0] != seq {
                        continue;
                    }
                    acked += read_buf[2] as usize + read_buf[3] as usize * 256;
                    if acked == block.len() {
                        break;
                    }
                }
            }

            Ok(())
        })
    }


}
