use crate::bit_iterator::BitIterator;

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
    let data = String::new();
    Block {
      is_last,
      encoding,
      data,
    }
  }
}

#[derive(Debug)]
pub struct Block {
  pub is_last: bool,
  encoding: HuffmanEncoding,
  data: String,
}

#[derive(Debug)]
enum HuffmanEncoding {
  Fixed,
  Dynamic,
}
