use std::io::{Seek, Write};

// 0000   ae c9 06 82 01 67 00 1c 42 6c 99 69 08 00 45 00   .....g..Bl.i..E.
// 0010   00 38 da ec 00 00 80 11 00 00 0a d3 37 03 0a d3   .8..........7...
// 0020   37 02 68 70 23 29 00 24 83 e0 00 00 00 00 00 00   7.hp#).$........
// 0030   08 00 91 cc ae 0c 00 00 00 00 08 00 01 00 40 85   ..............@.
// 0040   d2 9e 6c d6 d9 b2                                 ..l...
pub fn connect_response<W: Write + Seek>(writer: &mut W, cookie: u8) {
    println!("connect_response, cookie is {}", cookie);

    // WIP: Should be a C2S packet with a ulong cookie
    let x = vec![
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x91, 0xcc, 0xae, 0x0c, 0x00, 0x00, 0x00,
        0x00, 0x08, 0x00, 0x01, 0x00, 0x40, 0x85, 0xd2, 0x9e, 0x6c, 0xd6, 0xd9, 0xb2,
    ];

    writer.write(x.as_ref()).unwrap();
}
