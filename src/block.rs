use crate::bit_iterator::BitIterator;
use crate::huffman::HuffmanNode;

pub struct BlockReader<'a, I: Iterator<Item = &'a u8>> {
  bits: BitIterator<'a, I>,
}

impl<'a, I: Iterator<Item = &'a u8>> BlockReader<'a, I> {
  pub fn new(bits: BitIterator<'a, I>) -> BlockReader<'a, I> {
    BlockReader { bits }
  }

  // TODO - it is possible for a block to refer to data in a previous block, so
  // this needs to keep track of the entire read data so far, i.e. add an extra
  // param here to hold "data_so_far" or similar
  pub fn read_block(&mut self) -> Block {
    let is_last = self.bits.read_bits_inv(1) == 1;
    let encoding = match self.bits.read_bits_inv(2) {
      1 => HuffmanEncoding::Fixed,
      2 => HuffmanEncoding::Dynamic,
      _ => panic!("Unexpected block encoding"),
    };
    let (data, decode_items) = match encoding {
      HuffmanEncoding::Fixed => self.decode_block_data(HuffmanNode::fixed(), None),
      _ => (vec![], vec![]),
    };
    Block {
      is_last,
      encoding,
      data,
      decode_items,
    }
  }

  fn decode_block_data(
    &mut self,
    literals_root: HuffmanNode,
    distances_root: Option<HuffmanNode>,
  ) -> (Vec<u8>, Vec<DecodeItem>) {
    let mut data = vec![];
    let mut decode_items = vec![];
    let mut cur_literals = vec![];
    loop {
      let val = literals_root.decode_stream(&mut self.bits);
      match val {
        Some(x) if x < 256 => {
          data.push(x as u8);
          cur_literals.push(x as u8);
        }
        Some(256) => {
          // Stop
          if !cur_literals.is_empty() {
            decode_items.push(DecodeItem::Literal(cur_literals.clone()));
            cur_literals.clear();
          }
          break;
        }
        Some(x) if x < 285 => {
          // figure out length,distance
          // copy
          let length = self.decode_length(x);
          let distance = match distances_root {
            Some(_node) => unimplemented!("not yet implemented distances_root"),
            None => self.decode_fixed_distance(),
          };

          // copy data
          let mut copied_data = vec![];
          let v_idx = data.len() - distance as usize;
          for i in 0..length {
            let val = data[v_idx + i as usize];
            copied_data.push(val);
            data.push(val);
          }

          // Append to decode stream
          if !cur_literals.is_empty() {
            decode_items.push(DecodeItem::Literal(cur_literals.clone()));
            cur_literals.clear();
          }
          decode_items.push(DecodeItem::Match(length, distance));
        }
        Some(x) => panic!("Unexpected decoded value {}", x),
        None => panic!("Failed to decode from stream"),
      }
    }
    (data, decode_items)
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
      5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073,
      4097, 6145, 8193, 12289, 16385, 24577,
    ];

    // This is the only place that we read the bits in LSB->MSB order and *don't* invert them
    // The RFC says: "The extra bits should be interpreted as a machine integer stored with the most-significant bit first, e.g., bits 1110 represent the value 14."
    // So that appears to explain why the bits would be read in a non-inverted way. Of course...why!?
    let code = self.bits.read_bits(5);

    if code <= 3 {
      code + 1 // minimum distance is 1, so code 0 => distance 1
    } else {
      let extra_bits_to_read = (code as u8 - 2) / 2;
      let extra_dist = self.bits.read_bits_inv(extra_bits_to_read);
      extra_dist + EXTRA_DIST_ADDEND[code as usize - 4]
    }
  }
}

#[derive(Debug)]
pub struct Block {
  pub is_last: bool,
  pub encoding: HuffmanEncoding,
  pub data: Vec<u8>,
  pub decode_items: Vec<DecodeItem>,
}

impl Block {
  pub fn as_string(&self) -> String {
    match String::from_utf8(self.data.to_vec()) {
      Ok(s) => s,
      _ => String::from("<binary data>"),
    }
  }
}

#[derive(Debug)]
pub enum DecodeItem {
  Literal(Vec<u8>),
  Match(u32, u32),
}

use std::fmt;
impl fmt::Display for DecodeItem {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      DecodeItem::Literal(bytes) => {
        let string = match String::from_utf8(bytes.to_vec()) {
          Ok(v) => v,
          _ => String::from("<binary data>"),
        };
        write!(f, "literal '{}", string)
      }
      DecodeItem::Match(length, distance) => write!(f, "match {} {}", length, distance),
    }
  }
}

#[derive(Debug, PartialEq)]
pub enum HuffmanEncoding {
  Fixed,
  Dynamic,
}
