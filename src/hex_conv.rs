const fn byte_16_digits(byte: u8) -> [u8; 2] {
    [byte / 16, byte % 16]
}

#[test]
fn test_byte_16_digits() {
    assert_eq!(byte_16_digits(255), [15, 15]);
}

pub const fn byte_to_hex_digits(byte: u8) -> [u8; 2] {
    const TABLE: &[u8; 16] = b"0123456789ABCDEF";
    let [l, r] = byte_16_digits(byte);
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

fn digit_to_byte(digit: u8) -> u8 {
    match digit {
        b'0' => 0,
        b'1' => 1,
        b'2' => 2,
        b'3' => 3,
        b'4' => 4,
        b'5' => 5,
        b'6' => 6,
        b'7' => 7,
        b'8' => 8,
        b'9' => 9,
        b'a' | b'A' => 10,
        b'b' | b'B' => 11,
        b'c' | b'C' => 12,
        b'd' | b'D' => 13,
        b'e' | b'E' => 14,
        b'f' | b'F' => 15,
        _ => panic!("Invalid hex digit: {}", digit),
    }
}

pub fn merge_hex_halves(first: u8, second: u8) -> u8 {
    digit_to_byte(first) * 16 + digit_to_byte(second)
}

#[test]
fn test_merge_halves() {
    assert_eq!(merge_hex_halves(b'0', b'0'), 0);
    assert_eq!(merge_hex_halves(b'0', b'f'), 15);
    assert_eq!(merge_hex_halves(b'3', b'2'), 50);
    assert_eq!(merge_hex_halves(b'f', b'0'), 240);
    assert_eq!(merge_hex_halves(b'f', b'f'), 255);
}
