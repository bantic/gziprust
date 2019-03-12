pub struct Gzip {
  headers: Headers,
  crc32: u32,
  size: u32,
}

// impl Gzip {
//   fn new(bytes: Vec<u8>) -> Gzip {
//     let mut bytes = bytes.iter();
//     let headers = Headers::new(&mut bytes);
//     Gzip { headers }
//   }
// }

// Compression
// Flags
// MTIME
// "Extra Info" Flag
// OS

// Optional Extra Info
// Optional filename
// Optional comment
// Optional crc16
pub struct Headers {
  compression: Compression,
  mtime: u32,
  os: Os,
  filename: Option<String>,                 // indicated by flags bit 3
  comment: Option<String>,                  // indicated by flags bit 4
  crc16: Option<String>,                    // indicated by flags bit 1
  compressionInfo: Option<CompressionInfo>, // via "extra flags"
  isText: bool,                             // == Flags bit 0
  extraFields: Vec<ExtraField>,
}

const MAGIC_BYTES: (u8, u8) = (0x1f, 0x8b);

type ByteIterator = Iterator<Item = u8>;

// impl Headers {
//   fn new(bytes: &mut ByteIterator) -> Headers {
//     match (bytes.next(), bytes.next()) {
//       (Some(0x1f), Some(0x8b)) => (),
//       _ => panic!("Got wrong initial bytes"),
//     }

//     let byte = bytes.next().unwrap();
//     let compression = Compression::parse(byte);

//     let flags = bytes.next().unwrap();
//     let mtime = read_int(bytes, 4);
//   }
// }

fn read_int(bytes: &mut ByteIterator, size: usize) -> u32 {
  let mut values = vec![];
  while values.len() < size {
    let byte = bytes.next().unwrap();
    values.push(byte);
  }
  values
    .iter()
    .map(|&v| u32::from(v))
    .enumerate()
    .fold(0, |acc, (idx, val)| {
      dbg!((acc, idx, val));
      acc + dbg!(val << (8 * idx))
    })
}

enum Flags {
  Text = 0b1,
  CRC16 = 0b10,
  Extra = 0b100,
  FileName = 0b1000,
  Comment = 0b10000,
}

struct ExtraField {
  id: String,
  value: String,
}

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

pub enum CompressionInfo {
  MaximumCompressionSlowestAlgorithm,
  FastestAlgorithm,
}

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

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_read_int() {
    // let bytes = &[0b0, 0b0, 0b0, 0b0];
    // let mut bytes = bytes.iter().cloned();
    // assert_eq!(read_int(&mut bytes, 4), 0);

    let bytes = &[0b1, 0b0, 0b0, 0b0];
    let mut bytes = bytes.iter().cloned();
    assert_eq!(read_int(&mut bytes, 4), 1);

    let bytes = &[0b0, 0b1, 0b0, 0b0];
    let mut bytes = bytes.iter().cloned();
    assert_eq!(read_int(&mut bytes, 4), 256);

    let bytes = &[0b0, 0b0, 0b1, 0b0];
    let mut bytes = bytes.iter().cloned();
    assert_eq!(read_int(&mut bytes, 4), 0x0001_0000);

    let bytes = &[0b0, 0b0, 0b0, 0b1];
    let mut bytes = bytes.iter().cloned();
    assert_eq!(read_int(&mut bytes, 4), 0x0100_0000);

    let bytes = &[0b0000_0000, 0b1111_1111, 0b0000_0000, 0b0000_1000];
    let mut bytes = bytes.iter().cloned();
    assert_eq!(read_int(&mut bytes, 4), 0x0800_ff00);
  }
}
