#[repr(C)]
pub(super) struct Blob {
    pub(super) length: u32,
    pub(super) data: *mut u8,
}
