fn byte_16_digits(byte: u8) -> [u8; 2] {
    [byte / 16, byte % 16]
}

#[test]
fn test_byte_16_digits() {
    assert_eq!(byte_16_digits(255), [15, 15]);
}

pub fn byte_to_hex_digits(byte: u8) -> [u8; 2] {
    let [l, r] = byte_16_digits(byte);
    const TABLE: &[u8; 16] = b"0123456789ABCDEF";
    [TABLE[l as usize], TABLE[r as usize]]
}

#[test]
fn test_byte_to_hex_digits() {
    let pairs = [
        (255, b"FF"),
        (0, b"00"),
        (15, b"0F"),
        (16, b"10"),
        (154, b"9A"),
        (167, b"A7"),
        (6, b"06"),
        (64, b"40"),
    ];
    for (byte, hex) in pairs {
        assert_eq!(byte_to_hex_digits(byte), *hex);
    }
}
