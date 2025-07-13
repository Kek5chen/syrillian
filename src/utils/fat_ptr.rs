#[repr(C)]
pub struct FatPtr<'a, I> {
    pub data: &'a mut I,
    pub v_table: *const (),
}
