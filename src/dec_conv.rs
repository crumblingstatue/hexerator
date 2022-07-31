const fn byte_10_digits(byte: u8) -> [u8; 3] {
    [byte / 100, (byte % 100) / 10, byte % 10]
}

#[test]
fn test_byte_10_digits() {
    assert_eq!(byte_10_digits(255), [2, 5, 5]);
}

pub const fn byte_to_dec_digits(byte: u8) -> [u8; 3] {
    let [a, b, c] = byte_10_digits(byte);
    const TABLE: &[u8; 10] = b"0123456789";
    [TABLE[a as usize], TABLE[b as usize], TABLE[c as usize]]
}

#[test]
fn test_byte_to_dec_digits() {
    let pairs = [
        (255, b"255"),
        (0, b"000"),
        (1, b"001"),
        (15, b"015"),
        (16, b"016"),
        (154, b"154"),
        (167, b"167"),
        (6, b"006"),
        (64, b"064"),
        (127, b"127"),
        (128, b"128"),
        (129, b"129"),
    ];
    for (byte, hex) in pairs {
        assert_eq!(byte_to_dec_digits(byte), *hex);
    }
}
