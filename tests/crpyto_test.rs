use gromnie::crypto::magic_number::get_magic_number;

#[test]
fn test_get_magic_number() {
  assert_eq!(0, get_magic_number(vec![0, 0, 0, 0], 4));
}
