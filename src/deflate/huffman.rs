#[derive(Clone, Debug)]
pub struct TreeNode {
  len: u32,
  code: u32,
}

#[derive(Default, Debug, PartialOrd, PartialEq)]
pub struct HuffmanNode {
  code: Option<u32>,
  one: Option<Box<HuffmanNode>>,
  zero: Option<Box<HuffmanNode>>,
}

#[derive(Debug, PartialEq)]
pub enum HuffmanEncoding {
  Fixed,
  Dynamic,
}

impl HuffmanNode {
  pub fn decode_stream<I: Iterator<Item = bool>>(&self, bits: &mut I) -> Option<u32> {
    match self.code {
      Some(v) => Some(v),
      None => match bits.next() {
        Some(true) => match &self.one {
          Some(node) => node.decode_stream(bits),
          None => panic!("Unexpected decode_stream traversal"),
        },
        Some(false) => match &self.zero {
          Some(node) => node.decode_stream(bits),
          None => panic!("Unexpected decode_stream traversal"),
        },
        None => None,
      },
    }
  }

  pub fn from_code_lengths(code_lengths: &[u8]) -> HuffmanNode {
    let ranges = HuffmanRange::from_code_lengths(code_lengths);
    HuffmanNode::from_ranges(&ranges)
  }

  pub fn from_header_code_lengths(code_lengths: Vec<u8>) -> HuffmanNode {
    let ranges = HuffmanRange::from_header_code_keys(&code_lengths);
    HuffmanNode::from_ranges(&ranges)
  }

  fn from_ranges(ranges: &[HuffmanRange]) -> HuffmanNode {
    let range_len = ranges.len();
    let max_bit_length = ranges.iter().map(|range| range.bit_length).max().unwrap();

    let mut bitlength_count = vec![0; max_bit_length as usize + 1];
    for i in 0..range_len {
      if ranges[i].end == 0 && ranges[i].bit_length == 0 {
        break;
      }
      let mut to_add = ranges[i].end;
      if i > 0 {
        to_add -= ranges[i - 1].end;
      } else {
        to_add += 1;
      }
      bitlength_count[ranges[i].bit_length as usize] += to_add;
    }

    // determine first code of each bit length
    let mut code = 0;
    let mut next_code = vec![0; max_bit_length as usize + 1];
    for bits in 1..=max_bit_length {
      code = (code + bitlength_count[bits as usize - 1]) << 1;
      if bitlength_count[bits as usize] != 0 {
        next_code[bits as usize] = code;
      }
    }

    // build the code table
    let mut tree = vec![TreeNode { code: 0, len: 0 }; (ranges[range_len - 1].end + 1) as usize];
    let mut active_range = 0;
    for n in 0..=(ranges[range_len - 1].end) {
      if n > ranges[active_range].end {
        active_range += 1;
      }
      if ranges[active_range].bit_length != 0 {
        tree[n as usize].len = ranges[active_range].bit_length.into();
        if tree[n as usize].len != 0 {
          tree[n as usize].code = next_code[tree[n as usize].len as usize];
          next_code[tree[n as usize].len as usize] += 1;
        }
      }
    }

    // build the tree
    let mut root = HuffmanNode::default();
    for n in 0..=(ranges[range_len - 1 as usize].end) {
      let mut node = &mut root;
      if tree[n as usize].len != 0 {
        let mut bits = tree[n as usize].len;
        while bits > 0 {
          if tree[n as usize].code & (1 << (bits - 1)) != 0 {
            if node.one.is_none() {
              node.one = Some(Box::new(HuffmanNode::default()));
            }
            node = node.one.as_mut().unwrap();
          } else {
            if node.zero.is_none() {
              node.zero = Some(Box::new(HuffmanNode::default()));
            }
            node = node.zero.as_mut().unwrap();
          }
          bits -= 1;
        }
        if node.code.is_some() {
          panic!("expected no-code");
        }
        node.code = Some(n);
      }
    }

    root
  }

  pub fn fixed() -> HuffmanNode {
    Self::from_ranges(&HuffmanRange::fixed())
  }
}

pub fn fixed_byte_bit_lengths() -> Vec<u8> {
  let mut byte_bit_lengths = vec![8; 144];
  byte_bit_lengths.extend_from_slice(&[9; 112]);
  assert_eq!(byte_bit_lengths[143], 8); // 0-143: 8 bits
  assert_eq!(byte_bit_lengths[144], 9); // 144-255: 9 bits
  assert_eq!(byte_bit_lengths[255], 9);
  assert_eq!(byte_bit_lengths.len(), 256);
  byte_bit_lengths
}

#[derive(Debug, Clone)]
pub struct HuffmanRange {
  pub end: u32,
  pub bit_length: u8,
}

impl HuffmanRange {
  // These are hard-coded ranges, see
  // https://www.w3.org/Graphics/PNG/RFC-1951#fixed
  fn fixed() -> Vec<HuffmanRange> {
    vec![
      HuffmanRange {
        end: 143,
        bit_length: 8,
      },
      HuffmanRange {
        end: 255,
        bit_length: 9,
      },
      HuffmanRange {
        end: 279,
        bit_length: 7,
      },
      HuffmanRange {
        end: 287,
        bit_length: 8,
      },
    ]
  }

  fn from_code_lengths(lengths: &[u8]) -> Vec<HuffmanRange> {
    let mut ranges = vec![];
    let mut j = 0;
    for i in 0..lengths.len() {
      if i > 0 && lengths[i] != lengths[i - 1] {
        j += 1;
      }
      while ranges.len() < (j + 1) {
        ranges.push(HuffmanRange {
          end: 0,
          bit_length: 0,
        });
      }
      ranges[j].end = i as u32;
      ranges[j].bit_length = lengths[i];
    }

    ranges
  }

  // The DEFLATE spec defines a special ordering of the code lengths
  // for the initial, encoded huffman tree that is used to decode
  // the literals/lengths and distances trees
  fn from_header_code_keys(keys: &[u8]) -> Vec<HuffmanRange> {
    const HUFFMAN_LENGTH_OFFSETS: [usize; 19] = [
      16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut code_lengths = [0; 19];
    for i in 0..keys.len() {
      code_lengths[HUFFMAN_LENGTH_OFFSETS[i]] = keys[i];
    }

    HuffmanRange::from_code_lengths(&code_lengths)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  impl HuffmanNode {
    // Add the ability to decode a slice of bools
    // TODO - change the tests below to all use the public decode_stream
    fn decode(&self, bits: &[bool]) -> Option<u32> {
      match bits.len() {
        0 => self.code,
        _ => {
          if bits[0] {
            match &self.one {
              Some(node) => node.decode(&bits[1..]),
              None => None,
            }
          } else {
            match &self.zero {
              Some(node) => node.decode(&bits[1..]),
              None => None,
            }
          }
        }
      }
    }
  }

  // Turn a value into `len` bool bits,
  // it is used to pass values to the huffman tree
  pub fn to_bits(val: u32, len: usize) -> Vec<bool> {
    let mut result = vec![];
    let mut val = val;
    loop {
      result.push(val % 2 == 1);
      val >>= 1;
      if val == 0 {
        break;
      }
    }

    while result.len() < len {
      result.push(false);
    }
    result.reverse();
    result
  }

  #[test]
  fn test_to_bits() {
    let byte = 0;
    assert_eq!(to_bits(byte, 1), [false]);

    let byte = 1;
    assert_eq!(to_bits(byte, 1), [true]);

    let byte = 1;
    assert_eq!(to_bits(byte, 3), [false, false, true]);

    let byte = 0b000_0000;
    assert_eq!(
      to_bits(byte, 7),
      [false, false, false, false, false, false, false]
    );
  }

  #[test]
  fn test_fixed() {
    // See https://www.w3.org/Graphics/PNG/RFC-1951#fixed
    let root = HuffmanNode::fixed();
    assert_eq!(root.decode(&to_bits(0b0011_0000, 8)), Some(0));
    assert_eq!(root.decode(&to_bits(0b0011_1001, 8)), Some(9));
    assert_eq!(root.decode(&to_bits(0b1011_1111, 8)), Some(143));

    assert_eq!(root.decode(&to_bits(0b1100_0000, 8)), Some(280));
    assert_eq!(root.decode(&to_bits(0b1100_0010, 8)), Some(282));
    assert_eq!(root.decode(&to_bits(0b1100_0111, 8)), Some(287));

    assert_eq!(root.decode(&to_bits(0b000_0000, 7)), Some(256));
    assert_eq!(root.decode(&to_bits(0b000_0001, 7)), Some(257));
    assert_eq!(root.decode(&to_bits(0b000_0010, 7)), Some(258));
    assert_eq!(root.decode(&to_bits(0b000_0110, 7)), Some(262));
    assert_eq!(root.decode(&to_bits(0b000_1110, 7)), Some(270));
    assert_eq!(root.decode(&to_bits(0b001_0111, 7)), Some(279));

    assert_eq!(root.decode(&to_bits(0b1_1001_0000, 9)), Some(144));
    assert_eq!(root.decode(&to_bits(0b1_1111_1111, 9)), Some(255));
  }

  fn fixed_code_lengths() -> Vec<u8> {
    let mut code_lengths = Vec::with_capacity(287);
    for _i in 0..=143 {
      code_lengths.push(8);
    }
    for _i in 144..=255 {
      code_lengths.push(9);
    }
    for _i in 256..=279 {
      code_lengths.push(7);
    }
    for _i in 280..=287 {
      code_lengths.push(8);
    }
    code_lengths
  }

  #[test]
  fn test_generation_of_fixed_huffman_ranges() {
    assert_eq!(
      HuffmanNode::from_code_lengths(&fixed_code_lengths()),
      HuffmanNode::fixed()
    );
  }

  #[test]
  fn test_dynamic() {
    // Example table taken from https://commandlinefanatic.com/cgi-bin/showarticle.cgi?article=art001
    let keys = vec![6, 7, 7, 3, 3, 2, 3, 3, 4, 4, 5, 4];
    let root = HuffmanNode::from_header_code_lengths(keys);

    assert_eq!(root.decode(&to_bits(0b010, 3)), Some(0));
    assert_eq!(root.decode(&to_bits(0b1100, 4)), Some(4));
    assert_eq!(root.decode(&to_bits(0b1101, 4)), Some(5));
    assert_eq!(root.decode(&to_bits(0b011, 3)), Some(6));
    assert_eq!(root.decode(&to_bits(0b00, 2)), Some(7));
    assert_eq!(root.decode(&to_bits(0b01, 2)), None);
    assert_eq!(root.decode(&to_bits(0b100, 3)), Some(8));
    assert_eq!(root.decode(&to_bits(0b101, 3)), Some(9));
    assert_eq!(root.decode(&to_bits(0b110, 3)), None);
    assert_eq!(root.decode(&to_bits(0b1110, 4)), Some(10));
    assert_eq!(root.decode(&to_bits(0b11110, 5)), Some(11));
    assert_eq!(root.decode(&to_bits(0b11_1110, 6)), Some(16));
    assert_eq!(root.decode(&to_bits(0b111_1110, 7)), Some(17));
    assert_eq!(root.decode(&to_bits(0b111_1111, 7)), Some(18));
  }
}
