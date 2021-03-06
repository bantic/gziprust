pub struct BitIterator<I: Iterator<Item = u8>> {
  bytes: I,
  bitfield: Option<[bool; 8]>,
  cur_byte: u8,
  cur_idx: usize, // index of the bit within cur_byte
  done: bool,
  bit_buffer: Vec<bool>, // the bits read since the last time the buffer was flushed
}

impl<I: Iterator<Item = u8>> BitIterator<I> {
  pub fn new(bytes: I) -> Self {
    BitIterator {
      bytes,
      bitfield: None,
      cur_byte: 0,
      cur_idx: 0,
      bit_buffer: vec![],
      done: false,
    }
  }

  pub fn flush_buffer(&mut self) -> Vec<bool> {
    let result = self.bit_buffer.clone();
    self.bit_buffer.clear();
    result
  }

  #[allow(dead_code)]
  pub fn debug(&self) {
    let mut details = String::new();
    if let Some(bitfield) = self.bitfield {
      for (idx, &b) in bitfield.iter().enumerate() {
        let s_val = if b { "1" } else { "0" };
        if idx == self.cur_idx + 1 {
          details.push_str("<");
          details.push_str(s_val);
        } else {
          details.push_str(" ");
          details.push_str(s_val);
        }
      }
    }
    if self.cur_idx == 7 {
      details.push_str("<");
    }
    println!("[{:x}@{}  {}]", self.cur_byte, self.cur_idx, details);
  }

  pub fn read_bits_inv(&mut self, count: u8) -> u32 {
    let mut value = 0;
    for i in 0..count {
      let bit = match self.next() {
        Some(true) => 1,
        Some(false) => 0,
        None => panic!("Unexpected end of bits"),
      };
      self.bit_buffer.push(bit == 1);
      value |= bit << i;
    }
    value
  }

  pub fn read_bits(&mut self, count: u8) -> u32 {
    let mut value = 0;
    for i in 0..count {
      let bit = match self.next() {
        Some(true) => 1,
        Some(false) => 0,
        None => panic!("Unexpected end of bits"),
      };
      self.bit_buffer.push(bit == 1);
      value |= bit << (count - 1 - i);
    }
    value
  }

  pub fn discard_extra_bits(&mut self) {
    if self.done {
      return;
    }
    if self.cur_idx == 7 {
      return;
    }
    self.advance_byte();
  }

  // TODO add some tests to ensure this is done correctly
  fn advance_byte(&mut self) {
    if self.done {
      return;
    } else {
      match self.bytes.next() {
        Some(byte) => {
          self.cur_idx = 7;
          self.cur_byte = byte;
          self.bitfield = Some(byte_to_bits(byte));
        }
        None => {
          self.done = true;
        }
      };
    }
  }
}

impl<I: Iterator<Item = u8>> Iterator for BitIterator<I> {
  type Item = bool;
  fn next(&mut self) -> Option<Self::Item> {
    if self.done {
      return None;
    }

    // TODO clean up getting the first bitfield
    let bitfield = match self.bitfield {
      Some(bitfield) => bitfield,
      // Get first bitfield
      None => {
        self.advance_byte();
        if self.done {
          return None;
        } else {
          match self.bitfield {
            Some(bitfield) => bitfield,
            None => {
              return None;
            }
          }
        }
      }
    };

    let result = bitfield[self.cur_idx];
    self.bit_buffer.push(result);

    // Advance cur byte and cur index
    match self.cur_idx {
      0 => {
        self.advance_byte();
      }
      _ => {
        // decrement cur_idx
        self.cur_idx -= 1;
      }
    }
    Some(result)
  }
}

pub fn byte_to_bits(byte: u8) -> [bool; 8] {
  [
    byte & (1 << 7) != 0,
    byte & (1 << 6) != 0,
    byte & (1 << 5) != 0,
    byte & (1 << 4) != 0,
    byte & (1 << 3) != 0,
    byte & (1 << 2) != 0,
    byte & (1 << 1) != 0,
    byte & 1 != 0,
  ]
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_read_bits_inv_to_expected_file() {
    // This is taken verbatim from https://commandlinefanatic.com/cgi-bin/showarticle.cgi?article=art053#figure3_bottom
    let bytes = vec![0xbd, 0x1b, 0xfd, 0x6f, 0xda];
    let mut iter = BitIterator::new(bytes.into_iter());
    assert_eq!(iter.read_bits_inv(1), 1);
    assert_eq!(iter.read_bits_inv(2), 2);
    assert_eq!(iter.read_bits_inv(5), 23);
    assert_eq!(iter.read_bits_inv(5), 27);
    assert_eq!(iter.read_bits_inv(4), 8);
  }

  #[test]
  fn test_read_bits_inv() {
    let bytes = vec![0b0001_1000].into_iter();
    let mut iter = BitIterator::new(bytes);
    assert_eq!(iter.read_bits_inv(4), 8);
    assert_eq!(iter.read_bits_inv(4), 1);

    let bytes = vec![0b0101_1101].into_iter();
    let mut iter = BitIterator::new(bytes);
    assert_eq!(iter.read_bits_inv(5), 0b11101);
    assert_eq!(iter.read_bits_inv(3), 0b010);

    let bytes = vec![0b1].into_iter();
    let mut iter = BitIterator::new(bytes);
    assert_eq!(iter.read_bits_inv(1), 1);

    let bytes = vec![0b0].into_iter();
    let mut iter = BitIterator::new(bytes);
    assert_eq!(iter.read_bits_inv(1), 0);
  }

  #[test]
  fn test_read_bits() {
    let bytes = vec![0b0001_1000].into_iter();
    let mut iter = BitIterator::new(bytes);
    assert_eq!(iter.read_bits(4), 1);
    assert_eq!(iter.read_bits(4), 8);

    let bytes = vec![0b1101_1101].into_iter();
    let mut iter = BitIterator::new(bytes);
    assert_eq!(iter.read_bits(5), 0b10111);
    assert_eq!(iter.read_bits(3), 0b011);

    let bytes = vec![0b1].into_iter();
    let mut iter = BitIterator::new(bytes);
    assert_eq!(iter.read_bits_inv(1), 1);

    let bytes = vec![0b0].into_iter();
    let mut iter = BitIterator::new(bytes);
    assert_eq!(iter.read_bits_inv(1), 0);
  }

  #[test]
  fn test_byte_to_bits() {
    let byte = 0;
    assert_eq!(
      byte_to_bits(byte),
      [false, false, false, false, false, false, false, false]
    );

    let byte = 1;
    assert_eq!(
      byte_to_bits(byte),
      [false, false, false, false, false, false, false, true]
    );

    let byte = 0b1111_1111;
    assert_eq!(
      byte_to_bits(byte),
      [true, true, true, true, true, true, true, true]
    );

    let byte = 0b1010_0101;
    assert_eq!(
      byte_to_bits(byte),
      [true, false, true, false, false, true, false, true]
    );
  }
}
