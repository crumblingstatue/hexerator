use std::str::FromStr;

pub fn parse_offset(arg: &str) -> Result<usize, <usize as FromStr>::Err> {
    if let Some(stripped) = arg.strip_prefix("0x") {
        usize::from_str_radix(stripped, 16)
    } else if arg.contains(['a', 'b', 'c', 'd', 'e', 'f']) {
        usize::from_str_radix(arg, 16)
    } else {
        arg.parse()
    }
}
