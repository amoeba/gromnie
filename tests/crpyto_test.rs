use std::io::Cursor;

use gromnie::crypto::magic_number::get_magic_number;

#[test]
fn test_get_magic_number() {
  // Expected value came from debugging actestclient
  let input = vec![1, 2, 3];
  assert_eq!(0x1020300, get_magic_number(&input, 3, false));
}
