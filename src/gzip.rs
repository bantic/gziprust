use crate::bit_iterator::BitIterator;
use crate::block::{Block, BlockReader};

#[derive(Debug)]
pub struct Gzip {
  pub headers: Headers,
  pub blocks: Vec<Block>,
  pub crc32: u32,
  pub size: u32,
}

impl Gzip {
  pub fn new(bytes: Vec<u8>) -> Gzip {
    let mut bytes = bytes.iter();
    let headers = Headers::new(&mut bytes);

    // for byte in bytes {
    //   println!("next byte: {:x}", byte);
    // }

    let bit_iter = BitIterator::new(bytes);
    let mut block_reader = BlockReader::new(bit_iter);
    let mut blocks = vec![];
    loop {
      let block = block_reader.read_block();
      let is_last = block.is_last;
      blocks.push(block);
      if is_last {
        break;
      }
    }

    // This will read the size and crc32 (last 8 bytes)
    // let mut bytes = bytes.rev().take(8);
    // let size = read_int_be(&mut bytes, 4);
    // let crc32 = read_int_be(&mut bytes, 4);

    Gzip {
      headers,
      blocks,
      crc32: 0,
      size: 0,
    }
  }
}

// Compression
// Flags
// MTIME
// "Extra Info" Flag
// OS

// Optional Extra Info
// Optional filename
// Optional comment
// Optional crc16
#[derive(Debug)]
pub struct Headers {
  pub compression: Compression,
  pub mtime: u32,
  pub os: Os,
  pub filename: Option<String>, // indicated by flags bit 3
  pub comment: Option<String>,  // indicated by flags bit 4
  pub crc16: Option<u32>,       // indicated by flags bit 1
  pub compression_info: Option<CompressionInfo>, // via "extra flags"
  pub is_text: bool,            // == Flags bit 0
  pub extra_fields: Vec<ExtraField>,
}

// TODO -- I cannot figure out how to use
// this in the match below. `MAGIC_BYTES[0]` does not seem to be syntactically valid
const MAGIC_BYTES: [u8; 2] = [0x1f, 0x8b];

enum Flags {
  Text = 0b1,
  CRC16 = 0b10,
  Extra = 0b100,
  FileName = 0b1000,
  Comment = 0b10000,
}

impl Headers {
  fn new<'a, I: Iterator<Item = &'a u8>>(bytes: &mut I) -> Headers {
    match (bytes.next(), bytes.next()) {
      (Some(0x1f), Some(0x8b)) => (),
      _ => panic!("Got wrong initial bytes"),
    }

    let byte = bytes.next().unwrap();
    let compression = Compression::parse(*byte);

    let flags = bytes.next().unwrap();
    let mtime = read_int(bytes, 4);

    // todo use this
    let extra_flag = bytes.next().unwrap();
    let compression_info = CompressionInfo::parse(*extra_flag);

    let os = Os::parse(*bytes.next().unwrap());

    let extra_fields = if *flags & Flags::Extra as u8 != 0 {
      // parse extra fields
      let mut len = read_int(bytes, 2);
      let mut result = vec![];

      while len > 0 {
        let (bytes_read, field) = read_extra_data_field(bytes);
        len -= bytes_read;
        result.push(field);
      }
      result
    } else {
      vec![]
    };

    let filename = if *flags & Flags::FileName as u8 != 0 {
      Some(read_ascii_string(bytes))
    } else {
      None
    };

    let comment = if *flags & Flags::Comment as u8 != 0 {
      Some(read_ascii_string(bytes))
    } else {
      None
    };

    let is_text = *flags & Flags::Text as u8 != 0;

    let crc16 = if *flags & Flags::CRC16 as u8 != 0 {
      Some(read_int(bytes, 2))
    } else {
      None
    };

    Headers {
      compression,
      mtime,
      os,
      filename,
      comment,
      crc16,
      compression_info,
      is_text,
      extra_fields,
    }
  }
}

fn read_extra_data_field<'a, I: Iterator<Item = &'a u8>>(bytes: &mut I) -> (u32, ExtraField) {
  let mut id = String::new();
  id.push(*bytes.next().unwrap() as char);
  id.push(*bytes.next().unwrap() as char);

  let len = read_int(bytes, 2);
  let mut data = String::new();
  for _ in 0..len {
    data.push(*bytes.next().unwrap() as char);
  }

  (len + 4, ExtraField { id, data })
}

// Read little-endian int of `size` bytes
fn read_int<'a, I: Iterator<Item = &'a u8>>(bytes: &mut I, size: usize) -> u32 {
  let mut values = vec![];
  while values.len() < size {
    let byte = bytes.next().unwrap();
    values.push(byte);
  }
  values
    .iter()
    .map(|&v| u32::from(*v))
    .enumerate()
    .fold(0, |acc, (idx, val)| acc + (val << (8 * idx)))
}

// Read big-endian int of `size` bytes
fn read_int_be<'a, I: Iterator<Item = &'a u8>>(bytes: &mut I, size: usize) -> u32 {
  let mut values = vec![];
  while values.len() < size {
    let byte = bytes.next().unwrap();
    values.push(byte);
  }
  values
    .iter()
    .rev()
    .map(|&v| u32::from(*v))
    .enumerate()
    .fold(0, |acc, (idx, val)| acc + (val << (8 * idx)))
}

// Read null-terminated string
fn read_ascii_string<'a, I: Iterator<Item = &'a u8>>(bytes: &mut I) -> String {
  let mut result = String::new();
  loop {
    match bytes.next() {
      Some(b'\0') => break,
      Some(&v) => result.push(v as char),
      None => break,
    }
  }
  result
}

#[derive(Debug)]
pub struct ExtraField {
  pub id: String,
  pub data: String,
}

#[derive(Debug)]
pub enum Compression {
  Deflate,
}

impl Compression {
  fn parse(byte: u8) -> Compression {
    match byte {
      8 => Compression::Deflate,
      _ => panic!("Unexpected CM byte {}", byte),
    }
  }
}

#[derive(Debug)]
pub enum CompressionInfo {
  MaximumCompressionSlowestAlgorithm,
  FastestAlgorithm,
}

impl CompressionInfo {
  fn parse(byte: u8) -> Option<CompressionInfo> {
    match byte {
      2 => Some(CompressionInfo::MaximumCompressionSlowestAlgorithm),
      4 => Some(CompressionInfo::FastestAlgorithm),
      _ => None,
    }
  }
}

#[derive(Debug)]
pub enum Os {
  FATFilesystem,
  Amiga,
  VMS,
  Unix,
  VMcMS,
  AtaritOS,
  HPFS,
  Macintosh,
  Zsystem,
  CPm,
  TOPS20,
  NTFS,
  QDOS,
  Acorn,
  Unknown,
}

impl Os {
  fn parse(byte: u8) -> Os {
    use Os::*;
    match byte {
      0 => FATFilesystem,
      1 => Amiga,
      2 => VMS,
      3 => Unix,
      4 => VMcMS,
      5 => AtaritOS,
      6 => HPFS,
      7 => Macintosh,
      8 => Zsystem,
      9 => CPm,
      10 => TOPS20,
      11 => NTFS,
      12 => QDOS,
      13 => Acorn,
      255 => Unknown,
      _ => panic!("Unexpected OS value {}", byte),
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_read_int() {
    // let bytes = &[0b0, 0b0, 0b0, 0b0];
    // let mut bytes = bytes.iter().cloned();
    // assert_eq!(read_int(&mut bytes, 4), 0);

    let bytes = [0b1, 0b0, 0b0, 0b0];
    let mut bytes = bytes.iter();
    assert_eq!(read_int(&mut bytes, 4), 1);

    let bytes = &[0b0, 0b1, 0b0, 0b0];
    let mut bytes = bytes.iter();
    assert_eq!(read_int(&mut bytes, 4), 256);

    let bytes = &[0b0, 0b0, 0b1, 0b0];
    let mut bytes = bytes.iter();
    assert_eq!(read_int(&mut bytes, 4), 0x0001_0000);

    let bytes = &[0b0, 0b0, 0b0, 0b1];
    let mut bytes = bytes.iter();
    assert_eq!(read_int(&mut bytes, 4), 0x0100_0000);

    let bytes = &[0b0000_0000, 0b1111_1111, 0b0000_0000, 0b0000_1000];
    let mut bytes = bytes.iter();
    assert_eq!(read_int(&mut bytes, 4), 0x0800_ff00);
  }
}
