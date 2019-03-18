use chrono::NaiveDateTime;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;

mod config;

use crate::config::Config;
use gziprust::gzip::Gzip;
use std::process;

fn print_gzip_info(gz: Gzip, config: Config) {
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

  if config.debug {
    for (i, block) in gz.blocks.into_iter().enumerate() {
      println!("==================================");
      println!(
        "Block {}: is_last? {}, encoding: {:?}",
        i, block.is_last, block.encoding
      );
      let string = match String::from_utf8(block.data) {
        Ok(v) => v,
        _ => String::from("<binary data>"),
      };
      for item in block.decode_items {
        println!("\t{}", item);
      }
      println!("\tdata: \"{}\"", string);
      println!("==================================");
    }
  }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
  let mut buf = vec![];
  let mut file = File::open(&config.filename)?;
  let num_read = file.read_to_end(&mut buf)?;
  println!("Read {} bytes from {}", num_read, &config.filename);
  let gzip = Gzip::new(buf);
  print_gzip_info(gzip, config);
  Ok(())
}

fn main() {
  let config = Config::new(env::args()).unwrap_or_else(|err| {
    eprintln!("Error reading args: {}", err);
    process::exit(1);
  });

  if let Err(e) = run(config) {
    eprint!("Error: {}", e);
    process::exit(1);
  }
}
