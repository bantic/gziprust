#[derive(Clone, Debug)]
pub struct TreeNode {
  len: u32,
  code: u32,
}

#[derive(Default, Debug)]
pub struct HuffmanNode {
  code: Option<u32>,
  one: Option<Box<HuffmanNode>>,
  zero: Option<Box<HuffmanNode>>,
}

impl HuffmanNode {
  pub fn decode(&self, bits: &[bool]) -> Option<u32> {
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

  pub fn from_range(ranges: &[HuffmanRange]) -> HuffmanNode {
    let range_len = ranges.len();
    let max_bit_length = ranges.iter().map(|range| range.bit_length).max().unwrap();

    let mut bitlength_count = vec![0; max_bit_length as usize + 1];
    for i in 0..range_len {
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

    println!("tree {:?}", tree);

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
            node = match &mut node.one {
              Some(one) => &mut *one,
              None => panic!("uh oh"),
            };
          } else {
            if node.zero.is_none() {
              node.zero = Some(Box::new(HuffmanNode::default()));
            }
            node = match &mut node.zero {
              Some(zero) => &mut *zero,
              None => panic!("uh oh"),
            };
          }
          bits -= 1;
        }
        if node.code.is_some() {
          panic!("expected no-code");
        }
        node.code = Some(n);
      }
    }

    println!(
      "building fixed huffman. max_bit_length: {:?}, bitlength_count: {:?}, next codes: {:?}",
      max_bit_length, bitlength_count, next_code
    );

    println!("the ranges is: {:?}", &ranges);
    println!(
      "decode 0b0000000 {:?}",
      root.decode(&vec![false, false, false, false, false, false, false])
    );

    root
  }
}

#[derive(Debug)]
pub struct HuffmanRange {
  end: u32,
  bit_length: u8,
}

impl HuffmanRange {
  pub fn fixed() -> Vec<HuffmanRange> {
    let mut ranges = vec![];
    ranges.push(HuffmanRange {
      end: 143,
      bit_length: 8,
    });
    ranges.push(HuffmanRange {
      end: 255,
      bit_length: 9,
    });
    ranges.push(HuffmanRange {
      end: 279,
      bit_length: 7,
    });
    ranges.push(HuffmanRange {
      end: 287,
      bit_length: 8,
    });
    ranges
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::bit_iterator::to_bits;

  #[test]
  fn test_fixed() {
    // See https://www.w3.org/Graphics/PNG/RFC-1951#fixed
    let root = HuffmanNode::from_range(&HuffmanRange::fixed());
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
}
