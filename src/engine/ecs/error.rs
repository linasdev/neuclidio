use crate::engine::ecs::paged_sparse_set::PagedSparseSetError;
use crate::error::NeuclidioError;

#[derive(Debug)]
pub enum EcsError {
    PagedSparseSetError(PagedSparseSetError),
}

impl From<PagedSparseSetError> for EcsError {
    fn from(value: PagedSparseSetError) -> Self {
        Self::PagedSparseSetError(value)
    }
}

impl From<PagedSparseSetError> for NeuclidioError {
    fn from(value: PagedSparseSetError) -> Self {
        EcsError::from(value).into()
    }
}
