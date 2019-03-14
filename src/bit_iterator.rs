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
}

fn to_bits(byte: u8) -> [bool; 8] {
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
          let bitfield = to_bits(*byte);
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
            self.bitfield = Some(to_bits(*byte));
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

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_to_bits() {
    let byte = 0;
    assert_eq!(
      to_bits(byte),
      [false, false, false, false, false, false, false, false]
    );

    let byte = 1;
    assert_eq!(
      to_bits(byte),
      [false, false, false, false, false, false, false, true]
    );

    let byte = 0b1111_1111;
    assert_eq!(
      to_bits(byte),
      [true, true, true, true, true, true, true, true]
    );

    let byte = 0b1010_0101;
    assert_eq!(
      to_bits(byte),
      [true, false, true, false, false, true, false, true]
    );
  }
}
