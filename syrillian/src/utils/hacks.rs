use slotmap::DenseSlotMap;
use std::slice;

#[allow(unused)]
pub trait DenseSlotMapDirectAccess<V> {
    fn as_slice(&self) -> &[V];
}

impl<K: slotmap::Key, V> DenseSlotMapDirectAccess<V> for DenseSlotMap<K, V> {
    #[inline]
    fn as_slice(&self) -> &[V] {
        let len = self.len();
        if len == 0 {
            return &[];
        }

        // SAFETY:
        // - iter() currently yields values.iter() so .next().1 is &values[0]
        // - DenseSlotMap is dense: len == values.len()
        // - internal Vec<V> storage is contiguous
        unsafe {
            let first: *const V = self.values().next().unwrap_or_else(|| {
                log::error!(
                    "failed to cast self.values.next() to a const ptr of V in DenseSlotMap in hacks.rs"
                );
                std::process::exit(1);
            });
            slice::from_raw_parts(first, len)
        }
    }
}
