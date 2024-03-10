use std::io::Cursor;

use gromnie::messages::login_request::login_request;


#[test]
fn test_message_login_request() {
    let account_name = "acservertracker";
    let password = "jj9h26hcsggc";
    // TODO: Find out how I can test that the above gives me this checksum
    // from my PCAP;
    // 0x05d00093u32

    let mut buf = Cursor::new(Vec::new());
    login_request(&mut buf, &account_name.to_owned(), &password.to_owned());

    let expected = vec![0, 1, 2, 3];
    assert_eq!(expected, buf.into_inner());
}
