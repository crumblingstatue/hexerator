#[expect(
    clippy::cast_precision_loss,
    reason = "This is just an approximation of data size"
)]
pub fn human_size(size: usize) -> String {
    human_bytes::human_bytes(size as f64)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "This is just an approximation of data size"
)]
pub fn human_size_u64(size: u64) -> String {
    human_bytes::human_bytes(size as f64)
}
