pub fn find_hex_string(
    hex_string: &str,
    haystack: &[u8],
    mut f: impl FnMut(usize),
) -> anyhow::Result<()> {
    let needle = parse_hex_string(hex_string)?;
    for offset in memchr::memmem::find_iter(haystack, &needle) {
        f(offset);
    }
    Ok(())
}

enum HexStringSepKind {
    Comma,
    Whitespace,
    Dense,
}

fn detect_hex_string_sep_kind(hex_string: &str) -> HexStringSepKind {
    if hex_string.contains(',') {
        HexStringSepKind::Comma
    } else if hex_string.contains(char::is_whitespace) {
        HexStringSepKind::Whitespace
    } else {
        HexStringSepKind::Dense
    }
}

fn chunks_2(input: &str) -> impl Iterator<Item = anyhow::Result<&str>> {
    input
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| std::str::from_utf8(pair).map_err(anyhow::Error::from))
}

pub fn parse_hex_string(hex_string: &str) -> anyhow::Result<Vec<u8>> {
    match detect_hex_string_sep_kind(hex_string) {
        HexStringSepKind::Comma => {
            hex_string.split(',').map(|tok| parse_hex_token(tok.trim())).collect()
        }
        HexStringSepKind::Whitespace => {
            hex_string.split_whitespace().map(parse_hex_token).collect()
        }
        HexStringSepKind::Dense => chunks_2(hex_string).map(|tok| parse_hex_token(tok?)).collect(),
    }
}

fn parse_hex_token(tok: &str) -> anyhow::Result<u8> {
    Ok(u8::from_str_radix(tok, 16)?)
}

#[test]
fn test_parse_hex_string() {
    assert_eq!(
        parse_hex_string("de ad be ef").unwrap(),
        vec![0xde, 0xad, 0xbe, 0xef]
    );
    assert_eq!(
        parse_hex_string("de, ad, be, ef").unwrap(),
        vec![0xde, 0xad, 0xbe, 0xef]
    );
    assert_eq!(
        parse_hex_string("deadbeef").unwrap(),
        vec![0xde, 0xad, 0xbe, 0xef]
    );
}
