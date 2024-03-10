pub fn get_magic_number(data: &Vec<u8>, size: u32, include_size: bool) -> u32 {
  let mut magic : u32 = 0;

  if include_size {
    magic += (size as u32) << 16;
  }

  // i is used in both for loops
  let mut i : u32 = 0;

  for _ in 0..(size/4) {
    magic += data[i as usize] as u32;
    i += 1;
  }

  let mut shift = 3;

  i = i * 4;

  for _ in i..size {
    magic += (data[i as usize] as u32) << (shift * 8);
    shift -= 1;
    i += 1;
  }

  magic
}
