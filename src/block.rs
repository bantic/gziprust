use crate::bit_iterator::BitIterator;
use crate::huffman::HuffmanNode;

pub struct BlockReader<'a, I: Iterator<Item = &'a u8>> {
  bits: BitIterator<'a, I>,
}

impl<'a, I: Iterator<Item = &'a u8>> BlockReader<'a, I> {
  pub fn new(bits: BitIterator<'a, I>) -> BlockReader<'a, I> {
    BlockReader { bits }
  }

  pub fn read_block(&mut self) -> Block {
    let is_last = self.bits.read_bits_inv(1) == 1;
    let encoding = match self.bits.read_bits_inv(2) {
      1 => HuffmanEncoding::Fixed,
      2 => HuffmanEncoding::Dynamic,
      _ => panic!("Unexpected block encoding"),
    };
    let data = match encoding {
      HuffmanEncoding::Fixed => self.decode_block_data(HuffmanNode::fixed(), None),
      _ => vec![],
    };
    Block {
      is_last,
      encoding,
      data,
    }
  }

  fn decode_block_data(
    &mut self,
    literals_root: HuffmanNode,
    distances_root: Option<HuffmanNode>,
  ) -> Vec<u8> {
    let mut data = vec![];
    loop {
      println!("before literals_root.decode_stream");
      self.bits.debug();
      let val = literals_root.decode_stream(&mut self.bits);
      println!("after literals_root.decode_stream, decoded {:?}", val);
      self.bits.debug();
      match val {
        Some(x) if x < 256 => data.push(x as u8),
        Some(256) => break,
        Some(x) if x < 285 => {
          // figure out length,distance
          // copy
          let length = self.decode_length(x);
          let distance = match distances_root {
            Some(_node) => unimplemented!("not yet implemented distances_root"),
            None => self.decode_fixed_distance(),
          };
          println!("<{},{}>", length, distance);
          let mut copied_data = vec![];
          let v_idx = data.len() - distance as usize;
          for i in 0..length {
            let val = data[v_idx + i as usize];
            copied_data.push(val);
            data.push(val);
          }
          println!(
            "<{},{}> => {}",
            length,
            distance,
            String::from_utf8(copied_data).expect("failed to decode copied data into utf8 string")
          );
        }
        Some(x) => panic!("Unexpected decoded value {}", x),
        None => panic!("Failed to decode from stream"),
      }
    }
    data
  }

  fn decode_length(&mut self, code: u32) -> u32 {
    const EXTRA_LENGTH_ADDEND: [u32; 20] = [
      11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227,
    ];

    const MAX_LENGTH: u32 = 258;

    match code {
      0..=256 => panic!("Unexpected code for length {}", code),
      257..=264 => code - 257 + 3,
      265..=284 => {
        let extra_bits = ((code - 261) / 4) as u8;
        let length = self.bits.read_bits_inv(extra_bits);
        length + EXTRA_LENGTH_ADDEND[code as usize - 265]
      }
      285 => MAX_LENGTH,
      _ => panic!("Unexpected code for length {}", code),
    }
  }

  fn decode_fixed_distance(&mut self) -> u32 {
    const EXTRA_DIST_ADDEND: [u32; 26] = [
      4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256, 384, 512, 768, 1024, 1536, 2048, 3072,
      4096, 6144, 8192, 12288, 16384, 24576,
    ];
    println!("decode_fixed_distance, before reading code");
    self.bits.debug();
    let code = dbg!(self.bits.read_bits(5));
    println!("decode_fixed_distance, after reading code");
    self.bits.debug();

    if code <= 3 {
      code + 1 // minimum distance is 1, so code 0 => distance 1
    } else {
      let extra_bits_to_read = (code as u8 - 2) / 2;
      println!(
        "decode_fixed_distance, before reading {} extra bits",
        extra_bits_to_read
      );
      self.bits.debug();
      let extra_dist = self.bits.read_bits(extra_bits_to_read);
      println!(
        "decode_fixed_distance, after reading {} extra bits: {}",
        extra_bits_to_read, extra_dist
      );
      self.bits.debug();
      1 + extra_dist + EXTRA_DIST_ADDEND[code as usize - 4]
    }
  }
}

#[derive(Debug)]
pub struct Block {
  pub is_last: bool,
  pub encoding: HuffmanEncoding,
  pub data: Vec<u8>,
}

impl Block {
  pub fn as_string(&self) -> String {
    match String::from_utf8(self.data.to_vec()) {
      Ok(s) => s,
      _ => String::from("<binary data>"),
    }
  }
}

#[derive(Debug, PartialEq)]
pub enum HuffmanEncoding {
  Fixed,
  Dynamic,
}
