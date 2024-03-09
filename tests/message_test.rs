use std::io::Cursor;

use gromnie::messages::login_request::login_request;

#[test]
fn test_message_login_request() {
    let account_name = "mudlurk";
    let password = "mosswart";

    let mut buffer: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    login_request(&mut buffer, &account_name, password);

    let expected = vec![
        0, 0, 0, 0, 0, 0, 1, 0, 147, 0, 208, 5, 0, 0, 0, 0, 64, 0, 0, 0, 4, 0,
        49, 56, 48, 50, 0, 0, 52, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 62, 184, 168,
        88, 28, 0, 97, 99, 115, 101, 114, 118, 101, 114, 116, 114, 97, 99, 107,
        101, 114, 58, 106, 106, 57, 104, 50, 54, 104, 99, 115, 103, 103, 99,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    assert_eq!(expected, buffer.into_inner());
}
