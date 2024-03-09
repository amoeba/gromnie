use std::io::Cursor;

use gromnie::messages::login_request::login_request;

#[test]
fn test_add() {
    let account_name = "mudlurk";
    let password = "mosswart";

    let mut buffer: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    login_request(&mut buffer, &account_name, password);

}
