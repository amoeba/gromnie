use byteorder::{ByteOrder, LittleEndian};

pub fn get_magic_number(buffer: &[u8], size: usize, include_size: bool) -> u32 {
    let mut magic: u32 = 0;

    if include_size {
        magic += (size as u32) << 16;
    }

    let mut i: i32 = 0;

    for _ in 0..(size / 4) {
        let start = (i * 4) as usize;
        magic = magic.wrapping_add(LittleEndian::read_u32(&buffer[start..(start + 4)]));
        i += 1;
    }

    let mut shift = 3;

    i = i * 4;

    for _ in i..(size as i32) {
        magic = magic.wrapping_add((buffer[i as usize] as u32) << (shift * 8));
        shift -= 1;
        i += 1;
    }

    magic
}
