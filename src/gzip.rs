use crate::deflate::{inflate, Block, DecodeItem};

#[derive(Debug)]
pub struct Gzip {
  pub headers: Headers,
  pub blocks: Vec<Block>,
  pub decode_items: Vec<DecodeItem>,
  pub data: Vec<u8>,
  pub crc32: u32,
  pub size: u32,
  calculated_crc32: u32,
}

impl Gzip {
  fn read_trailing_data(final_bytes: Vec<u8>) -> (u32, u32) {
    let mut final_bytes = final_bytes.into_iter();
    let crc32 = read_int(&mut final_bytes, 4);
    let size = read_int(&mut final_bytes, 4);
    (crc32, size)
  }

  pub fn new(bytes: Vec<u8>) -> Gzip {
    let (crc32, size) = Gzip::read_trailing_data(bytes[(bytes.len() - 8)..].to_vec());

    let mut bytes = bytes.into_iter();
    let headers = Headers::new(&mut bytes);
    let inflate_result = inflate(&mut bytes);

    Gzip {
      headers,
      blocks: inflate_result.blocks,
      data: inflate_result.data,
      decode_items: inflate_result.decode_items,
      crc32,
      calculated_crc32: inflate_result.crc32,
      size,
    }
  }

  pub fn size_is_valid(&self) -> bool {
    self.data.len() as u64 % 2_u64.pow(32) == u64::from(self.size)
  }

  pub fn crc_is_valid(&self) -> bool {
    self.crc32 == self.calculated_crc32
  }

  pub fn as_string(&self) -> String {
    match String::from_utf8(self.data.clone()) {
      Ok(s) => s,
      _ => String::from("<binary data>"),
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

enum Flags {
  Text = 0b1,
  CRC16 = 0b10,
  Extra = 0b100,
  FileName = 0b1000,
  Comment = 0b10000,
}

impl Headers {
  fn new(bytes: &mut impl Iterator<Item = u8>) -> Headers {
    // TODO -- I cannot figure out how to use
    // this in the match below. `MAGIC_BYTES[0]` does not seem to be syntactically valid
    // const MAGIC_BYTES: [u8; 2] = [0x1f, 0x8b];
    const MAGIC_BYTE_1: u8 = 0x1f;
    const MAGIC_BYTE_2: u8 = 0x8b;

    match (bytes.next(), bytes.next()) {
      (Some(MAGIC_BYTE_1), Some(MAGIC_BYTE_2)) => (),
      _ => panic!("Got wrong initial bytes"),
    }

    let byte = bytes.next().unwrap();
    let compression = Compression::parse(byte);

    let flags = bytes.next().unwrap();
    let mtime = read_int(bytes, 4);

    let extra_flag = bytes.next().unwrap();
    let compression_info = CompressionInfo::parse(extra_flag);

    let os = Os::parse(bytes.next().unwrap());

    let extra_fields = if flags & Flags::Extra as u8 != 0 {
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

    let filename = if flags & Flags::FileName as u8 != 0 {
      Some(read_ascii_string(bytes))
    } else {
      None
    };

    let comment = if flags & Flags::Comment as u8 != 0 {
      Some(read_ascii_string(bytes))
    } else {
      None
    };

    let is_text = flags & Flags::Text as u8 != 0;

    let crc16 = if flags & Flags::CRC16 as u8 != 0 {
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

fn read_extra_data_field<I: Iterator<Item = u8>>(bytes: &mut I) -> (u32, ExtraField) {
  let mut id = String::new();
  id.push(bytes.next().unwrap() as char);
  id.push(bytes.next().unwrap() as char);

  let len = read_int(bytes, 2);
  let mut data = String::new();
  for _ in 0..len {
    data.push(bytes.next().unwrap() as char);
  }

  (len + 4, ExtraField { id, data })
}

// Read little-endian int of `size` bytes
fn read_int<I: Iterator<Item = u8>>(bytes: &mut I, size: usize) -> u32 {
  let mut values = vec![];
  while values.len() < size {
    let byte = bytes.next().unwrap();
    values.push(byte);
  }
  values
    .iter()
    .map(|&v| u32::from(v))
    .enumerate()
    .fold(0, |acc, (idx, val)| acc + (val << (8 * idx)))
}

// Read null-terminated string
fn read_ascii_string<I: Iterator<Item = u8>>(bytes: &mut I) -> String {
  let mut result = String::new();
  loop {
    match bytes.next() {
      Some(b'\0') | None => break,
      Some(v) => result.push(v as char),
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
  use crate::deflate::BlockEncoding;

  #[test]
  fn test_src_vs_compressed_in_dirs() {
    use std::fs;
    use std::fs::File;
    use std::io::Read;

    let src_dir = "tests/gzip/src/";
    let compressed_dir = "tests/gzip/compressed/";

    for entry in fs::read_dir(src_dir).expect("failed to read src dir") {
      let path = entry
        .expect("failed entry")
        .path()
        .into_os_string()
        .into_string()
        .expect("failed to convert path to string");
      let compressed_path = path.replace(src_dir, compressed_dir);

      let mut src = vec![];
      let mut file = File::open(path).expect("Failed to open file");
      file.read_to_end(&mut src).expect("Failed to read file");

      let mut compressed = vec![];
      let mut file = File::open(compressed_path).expect("Failed to open file");
      file
        .read_to_end(&mut compressed)
        .expect("Failed to read file");

      let gzip = Gzip::new(compressed);
      assert_eq!(gzip.data, src);
    }
  }

  #[test]
  fn test_crc32_in_dirs() {
    use std::fs;
    use std::fs::File;
    use std::io::Read;

    let src_dir = "tests/gzip/src/";
    let compressed_dir = "tests/gzip/compressed/";

    for entry in fs::read_dir(src_dir).expect("failed to read src dir") {
      let path = entry
        .expect("failed entry")
        .path()
        .into_os_string()
        .into_string()
        .expect("failed to convert path to string");
      let compressed_path = path.replace(src_dir, compressed_dir);

      let mut compressed = vec![];
      let mut file = File::open(compressed_path).expect("Failed to open file");
      file
        .read_to_end(&mut compressed)
        .expect("Failed to read file");

      let gzip = Gzip::new(compressed);
      assert!(gzip.crc_is_valid());
    }
  }

  mod dynamic_encoding {
    use super::*;

    #[test]
    fn gunzip_c_file_structure() {
      let bytes = include_bytes!("../tests/gzip/dynamic_encoding/gunzip.c.gz");
      let gzip = Gzip::new(bytes.to_vec());

      assert_eq!(gzip.blocks.len(), 1);
      assert_eq!(gzip.blocks[0].encoding, BlockEncoding::HuffmanDynamic);
    }
  }

  mod fixed_encoding {
    use super::*;

    #[test]
    fn gzip_distance_with_extra_bits() {
      // This file has fixed encoding, and a single match with a distance with an extra bit
      // The match is len 6, dist 7
      let bytes = include_bytes!("../tests/gzip/fixed_encoding/deflate-1flate.txt.gz");
      let gzip = Gzip::new(bytes.to_vec());

      assert_eq!(gzip.blocks.len(), 1);
      assert!(gzip.blocks[0].is_last);
      assert_eq!(gzip.blocks[0].encoding, BlockEncoding::HuffmanFixed);
    }

  }

  #[test]
  fn test_read_int() {
    let mut bytes = vec![0b0, 0b0, 0b0, 0b0].into_iter();
    assert_eq!(read_int(&mut bytes, 4), 0);

    let mut bytes = vec![0b1, 0b0, 0b0, 0b0].into_iter();
    assert_eq!(read_int(&mut bytes, 4), 1);

    let mut bytes = vec![0b0, 0b1, 0b0, 0b0].into_iter();
    assert_eq!(read_int(&mut bytes, 4), 256);

    let mut bytes = vec![0b0, 0b0, 0b1, 0b0].into_iter();
    assert_eq!(read_int(&mut bytes, 4), 0x0001_0000);

    let mut bytes = vec![0b0, 0b0, 0b0, 0b1].into_iter();
    assert_eq!(read_int(&mut bytes, 4), 0x0100_0000);

    let mut bytes = vec![0b0000_0000, 0b1111_1111, 0b0000_0000, 0b0000_1000].into_iter();
    assert_eq!(read_int(&mut bytes, 4), 0x0800_ff00);
  }
}
