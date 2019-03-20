mod bit_iterator;
mod huffman;

use bit_iterator::BitIterator;
use huffman::{fixed_byte_bit_lengths, HuffmanNode};

pub struct InflateResult {
  pub blocks: Vec<Block>,
  pub data: Vec<u8>,
}

pub fn inflate(bytes: &mut impl Iterator<Item = u8>) -> InflateResult {
  let bits = BitIterator::new(bytes);
  let mut block_reader = BlockReader::new(bits);
  let mut blocks = vec![];
  let mut data = vec![];

  loop {
    let block = block_reader.read_block();
    let is_last = block.is_last;
    data.extend_from_slice(&block.data);
    blocks.push(block);
    if is_last {
      break;
    }
  }

  InflateResult { blocks, data }
}

pub struct BlockReader<I: Iterator<Item = u8>> {
  bits: BitIterator<I>,
}

impl<I: Iterator<Item = u8>> BlockReader<I> {
  pub fn new(bits: BitIterator<I>) -> BlockReader<I> {
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
    let mut byte_bit_lengths;
    let (data, decode_items) = match encoding {
      HuffmanEncoding::Fixed => {
        byte_bit_lengths = fixed_byte_bit_lengths();
        self.decode_block_data(HuffmanNode::fixed(), None)
      }
      HuffmanEncoding::Dynamic => {
        let (literals_root, distances_root, _byte_bit_lengths) = self.decode_dynamic_data();
        byte_bit_lengths = _byte_bit_lengths;
        self.decode_block_data(literals_root, Some(distances_root))
      }
    };
    Block {
      is_last,
      encoding,
      data,
      decode_items,
      byte_bit_lengths,
    }
  }

  fn decode_dynamic_data(&mut self) -> (HuffmanNode, HuffmanNode, Vec<u8>) {
    let hlit = self.bits.read_bits_inv(5) as usize; // == # of lit/length codes - 257 (257-286)
    let hdist = self.bits.read_bits_inv(5) as usize; // == # of distance codes - 1 (1-32)
    let hclen = self.bits.read_bits_inv(4) as usize; // == # of code length codes - 4 (4-19)

    let mut code_length_code_lengths: Vec<u8> = Vec::with_capacity(3 * (4 + hclen) as usize);
    for _ in 0..(hclen + 4) {
      code_length_code_lengths.push(self.bits.read_bits_inv(3) as u8);
    }

    let code_lengths_tree = HuffmanNode::from_header_code_lengths(code_length_code_lengths);

    let mut alphabet_lens: Vec<u8> = vec![0; hlit + hdist + 258];
    let mut i = 0;
    while i < (hlit + hdist + 258) {
      if let Some(code) = code_lengths_tree.decode_stream(&mut self.bits) {
        assert!(code <= 19); // The code length encodings are all in the range 0-18
        let code = code as u8;
        match code {
          0..=15 => {
            alphabet_lens[i] = code;
            i += 1;
          }
          16..=18 => {
            let mut repeat_len = match code {
              16 => self.bits.read_bits_inv(2) + 3,
              17 => self.bits.read_bits_inv(3) + 3,
              18 => self.bits.read_bits_inv(7) + 11,
              _ => panic!("Unexpected code for repeat_len {}", code),
            };
            while repeat_len > 0 {
              repeat_len -= 1;
              alphabet_lens[i] = if code == 16 { alphabet_lens[i - 1] } else { 0 };
              i += 1;
            }
          }
          _ => panic!("Unexpected code {} encountered", code),
        }
      }
    }

    let byte_bit_lengths = alphabet_lens[0..255].to_vec();

    // build the literals ranges
    let literals_tree = HuffmanNode::from_code_lengths(&alphabet_lens[0..(hlit + 257)]);
    let distance_tree = HuffmanNode::from_code_lengths(&alphabet_lens[(hlit + 257)..]);
    (literals_tree, distance_tree, byte_bit_lengths)
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
      match literals_root.decode_stream(&mut self.bits) {
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
          let distance = self.decode_distance(&distances_root);

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
    assert!(code > 256);
    const EXTRA_LENGTH_ADDEND: [u32; 20] = [
      11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227,
    ];

    const MAX_LENGTH: u32 = 258;

    match code {
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

  fn decode_distance(&mut self, distances_root: &Option<HuffmanNode>) -> u32 {
    const EXTRA_DIST_ADDEND: [u32; 26] = [
      5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073,
      4097, 6145, 8193, 12289, 16385, 24577,
    ];
    let code = match distances_root {
      Some(node) => node.decode_stream(&mut self.bits).unwrap(),
      None => self.bits.read_bits(5),
    };
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
  pub byte_bit_lengths: Vec<u8>,
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
  Match(u32, u32), // length, distance
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
