pub(crate) mod pool;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub struct Entity {
    pub(crate) sparse_index: u32,
    pub(crate) generation: u32,
}
