use chrono::NaiveDateTime;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use structopt::StructOpt;

use gziprust::gzip::Gzip;

#[derive(Debug, StructOpt)]
#[structopt(
  name = "Gzip Decoder",
  about = "A tool for decoding and exploring Gzip'd files"
)]
pub struct Opt {
  /// Activate debug mode
  #[structopt(short = "d", long = "debug")]
  debug: bool,

  /// Write detailed byte-by-byte JSON data
  #[structopt(long = "json")]
  json: bool,

  /// Input .gz file
  #[structopt(parse(from_os_str))]
  input: PathBuf,

  /// Output file
  #[structopt(short = "o", long = "output", parse(from_os_str))]
  output: Option<PathBuf>,
}

fn print_gzip_info(gz: &Gzip) {
  println!("Gzip Info");
  println!("Compression: {:?}", gz.headers.compression);
  println!(
    "Modification Time: {} ({})",
    NaiveDateTime::from_timestamp(i64::from(gz.headers.mtime), 0),
    gz.headers.mtime,
  );
  println!("Os: {:?}", gz.headers.os);
  match &gz.headers.filename {
    Some(string) => println!("Original Filename: {}", string),
    None => println!("Original Filename: <unknown>"),
  }

  match &gz.headers.comment {
    Some(string) => println!("Comment: {}", string),
    None => println!("Comment: <none>"),
  }

  match &gz.headers.crc16 {
    Some(v) => println!("Headers CRC16: {}", v),
    None => println!("Headers CRC16: <none>"),
  }

  match &gz.headers.compression_info {
    Some(info) => println!("Compression info: {:?}", info),
    None => println!("Compression info: <none>"),
  }

  println!("Is Text Flag: {}", &gz.headers.is_text);
  println!("{} Extra Fields", &gz.headers.extra_fields.len());
  for extra_field in &gz.headers.extra_fields {
    println!("\t {}: {}", extra_field.id, extra_field.data);
  }

  println!(
    "Uncompressed data size: {} bytes (mod 2^32) {}",
    &gz.size,
    if gz.size_is_valid() { "✅" } else { "😲" }
  );
  println!(
    "CRC: {:x} {}",
    &gz.crc32,
    if gz.crc_is_valid() { "✅" } else { "😲" }
  );

  println!("Decompressed {} blocks", &gz.blocks.len());
}

fn print_debug_gzip_info(gz: &Gzip) {
  println!("Decompressed Data: {}", gz.as_string());

  for (i, block) in gz.blocks.iter().enumerate() {
    println!("==================================");
    println!(
      "Block {}: is_last? {}, encoding: {:?}",
      i, block.is_last, block.encoding
    );
    for item in &gz.decode_items {
      println!("\t{}", item);
    }
    println!("==================================");
  }
}

fn write_serialized_gzip(gz: &Gzip, buffer: std::fs::File) {
  serde_json::to_writer(buffer, &gz.decode_items).expect("failed to write serialize");
}

pub fn run(opts: Opt) -> Result<(), Box<dyn Error>> {
  let mut buf = vec![];
  let mut file = File::open(&opts.input)?;
  let num_read = file.read_to_end(&mut buf)?;
  println!("Read {} bytes from {:?}", num_read, &opts.input);
  let gzip = Gzip::new(buf);

  match opts.output {
    Some(path) => {
      let mut buffer = File::create(path).expect("Failed to open output path");
      if opts.json {
        write_serialized_gzip(&gzip, buffer);
      } else {
        write_data(&gzip, &mut buffer);
      }
    }
    None if opts.json => eprintln!("Must specify an output path if opts.json"),
    _ => (),
  }

  print_gzip_info(&gzip);

  if opts.debug {
    print_debug_gzip_info(&gzip);
  }

  Ok(())
}

fn write_data(gz: &Gzip, buffer: &mut std::fs::File) {
  buffer.write_all(&gz.data).expect("Failed");
}

fn main() {
  let opts = Opt::from_args();
  match run(opts) {
    Ok(()) => (),
    Err(e) => eprintln!("Error {}", e),
  };
}
