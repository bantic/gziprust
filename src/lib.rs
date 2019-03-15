use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;

pub mod gzip;
use crate::huffman::{HuffmanNode, HuffmanRange};
use gzip::Gzip;

mod bit_iterator;
mod block;
mod huffman;

pub struct Config {
  pub filename: String,
}

impl Config {
  pub fn new(mut args: env::Args) -> Result<Config, &'static str> {
    args.next();
    match args.next() {
      Some(filename) => Ok(Config { filename }),
      None => Err("Needs filename"),
    }
  }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
  let mut buf = vec![];
  let mut file = File::open(&config.filename)?;
  let num_read = file.read_to_end(&mut buf)?;
  println!("Read {} bytes from {}", num_read, &config.filename);
  let gzip = Gzip::new(buf);
  print_gzip_info(gzip);
  Ok(())
}

use chrono::NaiveDateTime;
fn print_gzip_info(gz: Gzip) {
  println!("Gzip Info");
  println!("Compression: {:?}", gz.headers.compression);
  println!(
    "Modification Time: {} ({})",
    NaiveDateTime::from_timestamp(i64::from(gz.headers.mtime), 0),
    gz.headers.mtime,
  );
  println!("Os: {:?}", gz.headers.os);
  match gz.headers.filename {
    Some(string) => println!("Original Filename: {}", string),
    None => println!("Original Filename: <unknown>"),
  }

  match gz.headers.comment {
    Some(string) => println!("Comment: {}", string),
    None => println!("Comment: <none>"),
  }

  match gz.headers.crc16 {
    Some(v) => println!("Headers CRC16: {}", v),
    None => println!("Headers CRC16: <none>"),
  }

  match gz.headers.compression_info {
    Some(info) => println!("Compression info: {:?}", info),
    None => println!("Compression info: <none>"),
  }

  println!("Is Text Flag: {}", gz.headers.is_text);
  println!("{} Extra Fields", gz.headers.extra_fields.len());
  for extra_field in gz.headers.extra_fields {
    println!("\t {}: {}", extra_field.id, extra_field.data);
  }
  println!("Uncompressed data size: {} bytes (mod 2^32)", gz.size);
  println!("CRC: {}", gz.crc32);

  println!("Decompressed {} blocks", gz.blocks.len());
  for (i, block) in gz.blocks.into_iter().enumerate() {
    println!("Block {}: {:?}", i, block);
    let data = String::from_utf8(block.data).expect("Failed to decode string");
    println!("Block data: \"{}\"", data);
  }

  HuffmanNode::from_range(&HuffmanRange::fixed());
}
