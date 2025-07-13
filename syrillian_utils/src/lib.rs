use std::fmt::Debug;

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

pub trait ShaderUniformSingleIndex: ShaderUniformIndex {
    fn first() -> Self {
        Self::by_index(0).expect("Shader uniform indexer was wrongfully declared as a single buffer indexer")
    }
}

pub trait ShaderUniformMultiIndex: ShaderUniformIndex {}