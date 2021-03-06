mod bit_iterator;
mod huffman;
pub use huffman::HuffmanEncoding;

use crate::crc32;
use bit_iterator::BitIterator;
use huffman::HuffmanNode;

struct Inflate<I: Iterator<Item = u8>> {
  result: InflateResult,
  bits: BitIterator<I>,
  cur_block_index: usize,
}

impl<I: Iterator<Item = u8>> Inflate<I> {
  fn new(bits: BitIterator<I>) -> Inflate<I> {
    Inflate {
      result: InflateResult::empty(),
      bits,
      cur_block_index: 0,
    }
  }

  fn inflate(&mut self) {
    self.cur_block_index = 0;
    loop {
      let block = self.read_block();
      self.cur_block_index += 1;
      let is_last = block.is_last;
      self.result.blocks.push(block);
      if is_last {
        break;
      }
    }
    self.result.crc32 = crc32::finalize(self.result.crc32)
  }

  fn read_block(&mut self) -> Block {
    let is_last = self.bits.read_bits_inv(1) == 1;
    let encoding = match self.bits.read_bits_inv(2) {
      0 => BlockEncoding::Stored,
      1 => BlockEncoding::HuffmanFixed,
      2 => BlockEncoding::HuffmanDynamic,
      v => unreachable!("Unexpected block encoding encountered: {}", v),
    };
    match encoding {
      BlockEncoding::HuffmanFixed => {
        self.decode_block_data(HuffmanNode::fixed(), None);
      }
      BlockEncoding::HuffmanDynamic => {
        let (literals_root, distances_root) = self.decode_dynamic_data();
        self.decode_block_data(literals_root, Some(distances_root));
      }
      BlockEncoding::Stored => {
        self.read_stored_block();
      }
    }
    Block { is_last, encoding }
  }

  fn decode_dynamic_data(&mut self) -> (HuffmanNode, HuffmanNode) {
    let hlit = self.bits.read_bits_inv(5) as usize; // == # of lit/length codes - 257 (257-286)
    let hdist = self.bits.read_bits_inv(5) as usize; // == # of distance codes - 1 (1-30)
    let hclen = self.bits.read_bits_inv(4) as usize; // == # of code length codes - 4 (4-19)

    const MAX_LEN_CODES: usize = 286;
    assert!(hlit < MAX_LEN_CODES);

    const MAX_DIST_CODES: usize = 30;
    assert!(hdist < MAX_DIST_CODES);
    let mut code_length_code_lengths: Vec<u8> = Vec::with_capacity(3 * (4 + hclen) as usize);
    for _ in 0..(hclen + 4) {
      let code = self.bits.read_bits_inv(3) as u8;
      code_length_code_lengths.push(code);
    }

    let code_lengths_tree = HuffmanNode::from_header_code_lengths(code_length_code_lengths);

    let mut alphabet_lens: Vec<u8> = vec![0; hlit + hdist + 258];
    let mut i = 0;
    while i < (hlit + hdist + 258) {
      if let Some(code) = code_lengths_tree.decode_stream(&mut self.bits) {
        assert!(code <= 18); // The code length encodings are all in the range 0-18
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

    // build the literals ranges
    let literals_tree = HuffmanNode::from_code_lengths(&alphabet_lens[0..(hlit + 257)]);
    let distance_tree = HuffmanNode::from_code_lengths(&alphabet_lens[(hlit + 257)..]);
    (literals_tree, distance_tree)
  }

  // TODO make the decodeitem output match that from infgen
  fn read_stored_block(&mut self) -> Vec<DecodeItem> {
    // skip to the next byte
    self.bits.discard_extra_bits();

    // Read 2-byte `len` value as LE
    let le = self.bits.read_bits_inv(8);
    let be = self.bits.read_bits_inv(8);
    let len: u32 = ((be << 8) | le) as u32;

    // Read 2-byte `nlen` value as LE
    // nlen is one's complement of len, see: https://www.w3.org/Graphics/PNG/RFC-1951#noncompressed
    let le = self.bits.read_bits_inv(8);
    let be = self.bits.read_bits_inv(8);
    let nlen: u32 = ((be << 8) | le) as u32;

    assert!(len == (!nlen & 0xFFFF));

    for _ in 0..len {
      let byte = self.bits.read_bits_inv(8) as u8;
      self.append_data(byte);
    }
    vec![]
  }

  fn push_literal(&mut self, byte: u8) {
    let data = DecodeData {
      bits: self.bits.flush_buffer(),
      block_id: self.cur_block_index,
    };
    self
      .result
      .decode_items
      .push(DecodeItem::Literal { value: byte, data });
    self.append_data(byte);
  }

  fn push_match(&mut self, length: u32, distance: u32) {
    assert!(self.result.data.len() > distance as usize);

    // Copy match to data
    let v_idx = self.result.data.len() - distance as usize;
    for i in 0..length {
      let val = self.result.data[v_idx + i as usize];
      self.append_data(val);
    }

    // push DecodeItem
    let data = DecodeData {
      bits: self.bits.flush_buffer(),
      block_id: self.cur_block_index,
    };
    self.result.decode_items.push(DecodeItem::Match {
      length,
      distance,
      data,
    });
  }

  fn append_data(&mut self, byte: u8) {
    self.result.data.push(byte);
    self.update_crc32(byte);
  }

  fn update_crc32(&mut self, byte: u8) {
    self.result.crc32 = crc32::update(self.result.crc32, byte);
  }

  fn decode_block_data(&mut self, literals_root: HuffmanNode, distances_root: Option<HuffmanNode>) {
    const MAX_LITERAL_CODE: u32 = 255;
    const STOP_CODE: u32 = 256;
    const MIN_DISTANCE_CODE: u32 = STOP_CODE + 1;
    const MAX_DISTANCE_CODE: u32 = 285;

    loop {
      self.bits.flush_buffer();
      match literals_root.decode_stream(&mut self.bits) {
        None => unreachable!("Failed to decode from stream"),
        Some(x) => match x {
          0..=MAX_LITERAL_CODE => {
            self.push_literal(x as u8);
          }
          STOP_CODE => {
            break;
          }
          MIN_DISTANCE_CODE..=MAX_DISTANCE_CODE => {
            let length = self.decode_length(x);
            let distance = self.decode_distance(&distances_root);

            self.push_match(length, distance);
          }
          _ => unreachable!("Unexpected decoded value {}", x),
        },
      }
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
}

pub struct InflateResult {
  pub blocks: Vec<Block>,
  pub decode_items: Vec<DecodeItem>,
  pub data: Vec<u8>,
  pub crc32: u32,
}

impl InflateResult {
  fn empty() -> InflateResult {
    InflateResult {
      blocks: vec![],
      data: vec![],
      decode_items: vec![],
      crc32: crc32::initial_value(),
    }
  }
}

pub fn inflate(bytes: &mut impl Iterator<Item = u8>) -> InflateResult {
  let bits = BitIterator::new(bytes);
  let mut inflator = Inflate::new(bits);
  inflator.inflate();
  inflator.result
}

#[derive(Debug, PartialEq)]
pub enum BlockEncoding {
  HuffmanFixed,
  HuffmanDynamic,
  Stored,
}

#[derive(Debug)]
pub struct Block {
  pub is_last: bool,
  pub encoding: BlockEncoding,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum DecodeItem {
  Literal {
    value: u8,
    data: DecodeData,
  },
  Match {
    length: u32,
    distance: u32,
    data: DecodeData,
  },
}

#[derive(Debug, Serialize)]
pub struct DecodeData {
  pub bits: Vec<bool>,
  pub block_id: usize,
}

use std::fmt;
impl fmt::Display for DecodeItem {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      DecodeItem::Literal { value: byte, data } => {
        let string = match byte {
          0x20..=0x7e => (*byte as char).to_string(),
          _ => format!("<{}>", byte),
        };
        write!(f, "literal {}, {:?}", string, data)
      }
      DecodeItem::Match {
        length,
        distance,
        data,
      } => write!(f, "match {} {} {:?}", length, distance, data),
    }
  }
}
