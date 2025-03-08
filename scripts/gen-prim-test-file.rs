#!/usr/bin/env -S cargo +nightly -Zscript

use std::{fs::File, io::Write};

fn main() {
    let mut f = File::create("test_files/primitives.bin").unwrap();
    macro_rules! prim {
        ($t:ident, $val:literal) => {
            let v: $t = $val;
            let mut buf = std::io::Cursor::new([0u8; 48]);
            // Write desc
            write!(&mut buf, "{} = {}", stringify!($t), v).unwrap();
            f.write_all(buf.get_ref()).unwrap();
            // Write byte repr
            // le
            buf.get_mut().fill(0);
            buf.get_mut()[..std::mem::size_of::<$t>()].copy_from_slice(&v.to_le_bytes());
            f.write_all(buf.get_ref()).unwrap();
            // be
            buf.get_mut().fill(0);
            buf.get_mut()[..std::mem::size_of::<$t>()].copy_from_slice(&v.to_be_bytes());
            f.write_all(buf.get_ref()).unwrap();
        };
    }
    prim!(u8, 42);
    prim!(i8, 42);
    prim!(u16, 4242);
    prim!(i16, 4242);
    prim!(u32, 424242);
    prim!(i32, 424242);
    prim!(u64, 424242424242);
    prim!(i64, 424242424242);
    prim!(u128, 424242424242424242424242);
    prim!(i128, 424242424242424242424242);
    prim!(f32, 42.4242);
    prim!(f64, 4242.42424242);
}
