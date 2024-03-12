use std::io::{Read, Seek};

pub fn get_magic_number<R: Read + Seek>(buffer: R, size: usize) -> u32{
  return 0;
}
