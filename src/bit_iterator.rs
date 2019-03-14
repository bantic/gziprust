pub struct BitIterator<'a, I: Iterator<Item = &'a u8>> {
  bytes: I,
  bitfield: Option<[bool; 8]>,
  cur_idx: usize,
  done: bool,
}

impl<'a, I: Iterator<Item = &'a u8>> BitIterator<'a, I> {
  pub fn new(bytes: I) -> BitIterator<'a, I> {
    BitIterator {
      bytes,
      bitfield: None,
      cur_idx: 0,
      done: false,
    }
  }

  pub fn read_bits_inv(&mut self, count: u8) -> u16 {
    let mut value = 0;
    for i in 0..count {
      let bit = if self.next().unwrap() { 1 } else { 0 };
      value |= bit << i;
    }
    value
  }
}

impl<'a, I: Iterator<Item = &'a u8>> Iterator for BitIterator<'a, I> {
  type Item = bool;
  fn next(&mut self) -> Option<Self::Item> {
    if self.done {
      return None;
    }

    let bitfield = match self.bitfield {
      Some(bitfield) => bitfield,
      // Get first bitfield
      None => match self.bytes.next() {
        Some(byte) => {
          self.cur_idx = 7;
          println!("getting first byte: {:x}", byte);
          let bitfield = byte_to_bits(*byte);
          self.bitfield = Some(bitfield);
          bitfield
        }
        None => {
          self.done = true;
          return None;
        }
      },
    };

    let result = bitfield[self.cur_idx];

    // Advance cur byte and cur index
    match self.cur_idx {
      0 => {
        // get next byte
        // reset cur_idx to 7
        println!("cur_idx is 0, getting next byte");
        match self.bytes.next() {
          Some(byte) => {
            println!("got next byte: {:x}", byte);
            self.bitfield = Some(byte_to_bits(*byte));
            self.cur_idx = 7;
          }
          None => {
            self.done = true;
          }
        }
      }
      _ => {
        println!("returning bit at index {}", self.cur_idx);
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

#[cfg(test)]
mod test {
  use super::*;

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
  fn test_read_bits_inv_to_expected_file() {
    // This is taken verbatim from https://commandlinefanatic.com/cgi-bin/showarticle.cgi?article=art053#figure3_bottom
    let bytes = vec![0xbd, 0x1b, 0xfd, 0x6f, 0xda];
    let mut iter = BitIterator::new(bytes.iter());
    assert_eq!(iter.read_bits_inv(1), 1);
    assert_eq!(iter.read_bits_inv(2), 2);
    assert_eq!(iter.read_bits_inv(5), 23);
    assert_eq!(iter.read_bits_inv(5), 27);
    assert_eq!(iter.read_bits_inv(4), 8);
  }
  #[test]
  fn test_read_bits_inv() {
    let bytes = vec![0b0001_1000];
    let mut iter = BitIterator::new(bytes.iter());
    assert_eq!(iter.read_bits_inv(4), 8);
    assert_eq!(iter.read_bits_inv(4), 1);

    let bytes = vec![0b0101_1101];
    let mut iter = BitIterator::new(bytes.iter());
    assert_eq!(iter.read_bits_inv(5), 0b11101);
    assert_eq!(iter.read_bits_inv(3), 0b010);
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
