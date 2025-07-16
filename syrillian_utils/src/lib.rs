use std::fmt::Debug;

/// Trait implemented by index enums used for uniform buffers.
pub trait ShaderUniformIndex: Debug + Sized {
    const MAX: usize;
    fn index(&self) -> u64;
    fn by_index(index: u64) -> Option<Self>;
    fn name() -> &'static str;

    // #[inline]
    // fn first() -> Self {
    //     Self::by_index(0).expect("Uniform index type doesn't have any buffers")
    // }
}

/// Marker trait for uniform index enums that only contain a single buffer.
pub trait ShaderUniformSingleIndex: ShaderUniformIndex {
    /// Returns the first and only buffer index.
    fn first() -> Self {
        Self::by_index(0).expect("Shader uniform indexer was wrongfully declared as a single buffer indexer")
    }
}

/// Marker trait for uniform index enums consisting of multiple buffers.
pub trait ShaderUniformMultiIndex: ShaderUniformIndex {}