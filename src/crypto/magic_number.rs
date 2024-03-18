use byteorder::{ByteOrder, LittleEndian};

pub fn get_magic_number(buffer: &[u8], size: usize, include_size: bool) -> u32{
  let mut magic : u32 = 0;

  if include_size {
    magic += (size as u32) << 16;
  }

  // i is used in both for loops
  let mut i : i32 = 0;

  for _ in 0..(size / 4) {
    let start = (i * 4) as usize;
    println!("current {}; next {}", magic, LittleEndian::read_u32(&buffer[start..(start+4)]));

    magic += LittleEndian::read_u32(&buffer[start..(start+4)]);

    i += 1;
  }

  let mut shift = 3;

  i = i * 4;

  for _ in i..(size as i32) {
    println!("current {}; next {}", magic, (buffer[i as usize] as u32) << (shift * 8));

    magic += (buffer[i as usize] as u32) << (shift * 8);
    shift -= 1;
    i += 1;
  }

  magic
}
