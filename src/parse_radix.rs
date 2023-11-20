use num_traits::Num;

pub fn parse_guess_radix<T: Num>(input: &str) -> Result<T, <T as Num>::FromStrRadixErr> {
    if let Some(stripped) = input.strip_prefix("0x") {
        T::from_str_radix(stripped, 16)
    } else if input.contains(['a', 'b', 'c', 'd', 'e', 'f']) {
        T::from_str_radix(input, 16)
    } else {
        T::from_str_radix(input, 10)
    }
}

/// Relativity of an offset
pub enum Relativity {
    Absolute,
    RelAdd,
    RelSub,
}

pub fn parse_offset_maybe_relative(
    input: &str,
) -> Result<(usize, Relativity), <usize as Num>::FromStrRadixErr> {
    Ok(if let Some(stripped) = input.strip_prefix('-') {
        (parse_guess_radix(stripped.trim_end())?, Relativity::RelSub)
    } else if let Some(stripped) = input.strip_prefix('+') {
        (parse_guess_radix(stripped.trim_end())?, Relativity::RelAdd)
    } else {
        (parse_guess_radix(input.trim_end())?, Relativity::Absolute)
    })
}
