use std::io::Cursor;

use gromnie::net::packets::login_request::LoginRequestPacket;

fn compare_arrays_with_wildcard(expected: &Vec<u8>, actual: &Vec<u8>) -> bool {
    if expected.len() != actual.len() {
        return false;
    }

    let pass = expected
        .iter()
        .zip(actual.iter())
        .all(|(&exp, &act)| exp == act || exp == 0xFF);

    // Hijack assert_eq! to print useful output when this fails. Otherwise this
    // just panics.
    if !pass {
        assert_eq!(expected, actual);
    } else {
        return true;
    }

    false
}

#[test]
fn test_message_login_request() {
    // Login with root:root, captured with aclog
    let pak_data_root_root = vec![
        0x00, 0x00, 0x00, 0x00, // Sequence
        0x00, 0x00, 0x01, 0x00, // Flags
        0xff, 0xff, 0xff, 0xff, // Checksum
        0x00, 0x00, // RecipientId
        0x00, 0x00, // TimeSinceLastPacket
        0x2d, 0x00, // Size
        0x00, 0x00, // Iteration
        0x04, 0x00, 0x31, 0x38, 0x30, 0x32, // ClientVersion
        0x00, 0x00, // Align
        0x21, 0x00, 0x00, 0x00, // Length
        0x02, 0x00, 0x00, 0x00, // AuthType
        0x00, 0x00, 0x00, 0x00, // AuthFlags
        0xff, 0xff, 0xff, 0xff, // Sequence
        0x04, 0x00, 0x72, 0x6f, 0x6f, 0x74, // AccountName
        0x00, 0x00, // Align
        0x00, 0x00, 0x00, 0x00, // ???
        0x05, 0x00, 0x00, 0x00, // Password WString (Length + 1)
        0x04, 0x72, 0x6f, 0x6f, 0x74, // Password
    ];

    let mut pak_root_root = LoginRequestPacket::new("root", "root");
    let mut buffer_root_root = Cursor::new(Vec::new());
    pak_root_root.serialize(&mut buffer_root_root);
    assert!(compare_arrays_with_wildcard(
        &pak_data_root_root,
        &buffer_root_root.into_inner()
    ));

    // Login with testing:testing, captured with aclog
    let pak_data_testing_testing = vec![
        0x00, 0x00, 0x00, 0x00, // Sequence
        0x00, 0x00, 0x01, 0x00, // Flags
        0xff, 0xff, 0xff, 0xff, // Checksum
        0x00, 0x00, // RecipientId
        0x00, 0x00, // TimeSinceLastPacket
        0x34, 0x00, // Size
        0x00, 0x00, // Iteration
        0x04, 0x00, 0x31, 0x38, 0x30, 0x32, // ClientVersion (Len + "1802")
        0x00, 0x00, // Align
        0x28, 0x00, 0x00, 0x00, // Length
        0x02, 0x00, 0x00, 0x00, // AuthType
        0x00, 0x00, 0x00, 0x00, // AuthFlags
        0xff, 0xff, 0xff, 0xff, // Sequence
        0x07, 0x00, 0x74, 0x65, 0x73, 0x74, 0x69, 0x6e, 0x67,
        0x00, // Account (Len + "testing")
        0x00, 0x00, // Align
        0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, // Password WString (Length + 1)
        0x07, 0x74, 0x65, 0x73, 0x74, 0x69, 0x6e, 0x67, // Passowrd (Len + "testing")
    ];

    let mut pak_testing_testing = LoginRequestPacket::new("testing", "testing");
    let mut buffer_testing_testing = Cursor::new(Vec::new());
    pak_testing_testing.serialize(&mut buffer_testing_testing);
    assert!(compare_arrays_with_wildcard(
        &pak_data_testing_testing,
        &buffer_testing_testing.into_inner()
    ));

    // Logging in with elevencharsl:elevencharsl, captured with aclog
    // Not actually elevencharslong
    let pak_data_elevencharsl_elevencharsl = vec![
        0x00, 0x00, 0x00, 0x00, // Sequence
        0x00, 0x00, 0x01, 0x00, // Flags
        0xff, 0xff, 0xff, 0xff, // Checksum
        0x00, 0x00, // RecipientId
        0x00, 0x00, // TimeSinceLastPacket
        0x3d, 0x00, // Size
        0x00, 0x00, // Iteration
        0x04, 0x00, 0x31, 0x38, 0x30, 0x32, // ClientVersion
        0x00, 0x00, // Align
        0x31, 0x00, 0x00, 0x00, // Length
        0x02, 0x00, 0x00, 0x00, // AuthType
        0x00, 0x00, 0x00, 0x00, // AuthFlags
        0xff, 0xff, 0xff, 0xff, // Sequence
        0x0c, 0x00, 0x65, 0x6c, 0x65, 0x76, 0x65, 0x6e, 0x63, 0x68, 0x61, 0x72, 0x73,
        0x6c, // AccountName
        0x00, 0x00, // Align
        0x00, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x00, 0x00, // Password WString (Length + 1)
        0x0c, 0x65, 0x6c, 0x65, 0x76, 0x65, 0x6e, 0x63, 0x68, 0x61, 0x72, 0x73,
        0x6c, // Password
    ];

    let mut pak_elevencharsl_elevencharsl = LoginRequestPacket::new("elevencharsl", "elevencharsl");
    let mut buffer_elevencharsl_elevencharsl = Cursor::new(Vec::new());
    pak_elevencharsl_elevencharsl.serialize(&mut buffer_elevencharsl_elevencharsl);
    assert!(compare_arrays_with_wildcard(
        &pak_data_elevencharsl_elevencharsl,
        &buffer_elevencharsl_elevencharsl.into_inner()
    ));
}
