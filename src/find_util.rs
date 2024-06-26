pub fn find_hex_string(
    hex_string: &str,
    haystack: &[u8],
    mut f: impl FnMut(usize),
) -> anyhow::Result<()> {
    let needle = hex_string
        .split_whitespace()
        .map(|s| u8::from_str_radix(s, 16))
        .collect::<Result<Vec<_>, _>>()?;
    for offset in memchr::memmem::find_iter(haystack, &needle) {
        f(offset);
    }
    Ok(())
}
