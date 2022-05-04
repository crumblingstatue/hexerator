pub trait SliceExt {
    fn pattern_fill(&mut self, pattern: &Self);
}

impl<T: Copy> SliceExt for [T] {
    fn pattern_fill(&mut self, pattern: &Self) {
        for (src, dst) in pattern.iter().cycle().zip(self.iter_mut()) {
            *dst = *src;
        }
    }
}

#[test]
fn test_pattern_fill() {
    let mut buf = [0u8; 10];
    buf.pattern_fill(b"foo");
    assert_eq!(&buf, b"foofoofoof");
    buf.pattern_fill(b"Hello, World!");
    assert_eq!(&buf, b"Hello, Wor");
}
