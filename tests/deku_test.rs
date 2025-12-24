use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
struct DekuTest {
    field_a: u32,
    field_b: u32,
    field_c: u32,
}

#[test]
fn test_deku_works_as_i_think_it_does() {
    let data: Vec<u8> = vec![
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0xD3, 0xF5, 0x08, 0xA2,
    ];
    let (_rest, mut val) = DekuTest::from_bytes((data.as_ref(), 0)).unwrap();
    assert_eq!(
        DekuTest {
            field_a: 0x0,
            field_b: 65536,
            field_c: 2718496211,
        },
        val
    );
}
