use std::str::FromStr;

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

pub fn parse_offset(arg: &str) -> Result<usize, <usize as FromStr>::Err> {
    parse_guess_radix::<usize>(arg)
}
