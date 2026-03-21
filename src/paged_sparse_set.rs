use crate::error::NeuclidioResult;
use crate::generational_free_list::GenerationalIndex;
use downcast_rs::{Downcast, impl_downcast};

#[derive(Debug)]
pub enum PagedSparseSetError {
    SlotFull,
}

pub trait PagedSparseSetExt<H: GenerationalIndex>: Downcast {
    fn remove(&mut self, sparse_index: u32) -> bool;
    fn contains(&self, sparse_index: u32) -> bool;
    fn len(&self) -> usize;
    fn get_item_handle_by_dense_index(&self, dense_index: usize) -> Option<H>;
}

impl_downcast!(PagedSparseSetExt<H> where H: GenerationalIndex);

pub struct PagedSparseSet<const PAGE_SIZE: usize, H: GenerationalIndex, T> {
    sparse_pages: Vec<Option<Box<SparseSetPage<PAGE_SIZE>>>>,
    dense: Vec<(H, T)>,
}

pub struct SparseSetPage<const PAGE_SIZE: usize> {
    indices: [Option<usize>; PAGE_SIZE],
}

impl<const PAGE_SIZE: usize, H: GenerationalIndex + 'static, T: 'static> PagedSparseSetExt<H>
    for PagedSparseSet<PAGE_SIZE, H, T>
{
    fn remove(&mut self, sparse_index: u32) -> bool {
        self.take(sparse_index).is_some()
    }

    fn contains(&self, sparse_index: u32) -> bool {
        let (sparse_page_index, sparse_item_index) = Self::get_page_and_item_indices(sparse_index);

        if let Some(Some(sparse_page)) = self.sparse_pages.get(sparse_page_index) {
            sparse_page.indices[sparse_item_index].is_some()
        } else {
            false
        }
    }

    fn len(&self) -> usize {
        self.dense.len()
    }

    fn get_item_handle_by_dense_index(&self, dense_index: usize) -> Option<H> {
        self.dense
            .get(dense_index)
            .map(|(item_handle, _)| *item_handle)
    }
}

impl<const PAGE_SIZE: usize, H: GenerationalIndex + 'static, T: 'static>
    PagedSparseSet<PAGE_SIZE, H, T>
{
    const ASSERT_PAGE_SIZE_POWER_OF_TWO: () = assert!(
        PAGE_SIZE.is_power_of_two(),
        "PAGE_SIZE must be a power of two"
    );
    const PAGE_INDEX_BITSHIFT: usize = PAGE_SIZE.trailing_zeros() as usize;
    const ITEM_INDEX_BITMASK: usize = PAGE_SIZE - 1;

    pub fn new() -> Self {
        let _ = Self::ASSERT_PAGE_SIZE_POWER_OF_TWO;

        Self {
            sparse_pages: vec![],
            dense: vec![],
        }
    }

    // TODO: Implement a maintain function which would remove unused pages

    pub fn insert(&mut self, item_handle: H, item: T) -> NeuclidioResult<()> {
        let (sparse_page_index, sparse_item_index) =
            Self::get_page_and_item_indices(item_handle.sparse_index());

        if sparse_page_index >= self.sparse_pages.len() {
            self.sparse_pages
                .resize_with(sparse_page_index + 1, || None);
        }

        let sparse_page = self.sparse_pages[sparse_page_index]
            .get_or_insert_with(|| Box::new(SparseSetPage::new()));

        if sparse_page.indices[sparse_item_index].is_some() {
            return Err(PagedSparseSetError::SlotFull.into());
        }

        let dense_index = self.dense.len();
        self.dense.push((item_handle, item));

        sparse_page.indices[sparse_item_index].replace(dense_index);
        Ok(())
    }

    pub fn take(&mut self, sparse_index: u32) -> Option<(H, T)> {
        let (sparse_page_index, sparse_item_index) = Self::get_page_and_item_indices(sparse_index);

        let sparse_page =
            if let Some(Some(sparse_page)) = self.sparse_pages.get_mut(sparse_page_index) {
                sparse_page
            } else {
                return None;
            };

        let dense_index = if let Some(dense_index) = sparse_page.indices[sparse_item_index].take() {
            dense_index
        } else {
            return None;
        };

        let (item_handle, item) = self.dense.swap_remove(dense_index);

        if let Some((swapped_item_handle, _)) = self.dense.get(dense_index) {
            let (swapped_sparse_page_index, swapped_sparse_item_index) =
                Self::get_page_and_item_indices(swapped_item_handle.sparse_index());

            let swapped_sparse_page = self.sparse_pages[swapped_sparse_page_index]
                .as_mut()
                .unwrap();

            swapped_sparse_page.indices[swapped_sparse_item_index].replace(dense_index);
        }

        Some((item_handle, item))
    }

    pub fn iter(&self) -> impl Iterator<Item = (H, &T)> {
        self.dense
            .iter()
            .map(|(item_handle, item)| (*item_handle, item))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (H, &mut T)> {
        self.dense
            .iter_mut()
            .map(|(item_handle, item)| (*item_handle, item))
    }

    pub fn get(&self, sparse_index: u32) -> Option<&T> {
        let (sparse_page_index, sparse_item_index) = Self::get_page_and_item_indices(sparse_index);
        let sparse_page = self.sparse_pages.get(sparse_page_index)?.as_ref()?;
        let dense_index = sparse_page.indices[sparse_item_index]?;

        Some(&self.dense.get(dense_index).unwrap().1)
    }

    pub fn get_mut(&mut self, sparse_index: u32) -> Option<&mut T> {
        let (sparse_page_index, sparse_item_index) = Self::get_page_and_item_indices(sparse_index);
        let sparse_page = self.sparse_pages.get_mut(sparse_page_index)?.as_mut()?;
        let dense_index = sparse_page.indices[sparse_item_index]?;

        Some(&mut self.dense.get_mut(dense_index).unwrap().1)
    }

    fn get_page_and_item_indices(sparse_index: u32) -> (usize, usize) {
        let sparse_index = sparse_index as usize;
        let sparse_page_index = sparse_index >> Self::PAGE_INDEX_BITSHIFT;
        let sparse_item_index = sparse_index & Self::ITEM_INDEX_BITMASK;

        (sparse_page_index, sparse_item_index)
    }
}

impl<const PAGE_SIZE: usize> SparseSetPage<PAGE_SIZE> {
    pub fn new() -> Self {
        Self {
            indices: [None; PAGE_SIZE],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ecs::error::EcsError;
    use crate::error::NeuclidioError;
    use googletest::prelude::*;

    const PAGE_SIZE_FOR_TESTS: usize = 1024;

    #[derive(PartialEq, Eq, Copy, Clone, Debug)]
    struct TestHandle {
        sparse_index: u32,
        generation: u32,
    }

    impl TestHandle {
        pub fn new(sparse_index: u32) -> Self {
            Self::from_parts(sparse_index, 0).unwrap()
        }
    }

    impl GenerationalIndex for TestHandle {
        fn from_parts(sparse_index: u32, generation: u32) -> Option<Self> {
            Some(Self {
                sparse_index,
                generation,
            })
        }

        fn sparse_index(&self) -> u32 {
            self.sparse_index
        }

        fn generation(&self) -> u32 {
            self.generation
        }
    }

    // remove

    #[test]
    fn should_remove_item_when_there_is_only_one_item() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();

        // Execute
        let result = target.remove(0);

        // Assert
        assert_that!(result, eq(true));

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(each(none())),
        );
        assert_that!(target.dense, is_empty());
    }

    #[test]
    fn should_remove_item_when_there_are_multiple_items_in_the_same_page() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(1), 0x34).unwrap();

        // Execute
        let result = target.remove(0);

        // Assert
        assert_that!(result, eq(true));

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[1] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(target.dense, elements_are![eq(&(TestHandle::new(1), 0x34))]);
    }

    #[test]
    fn should_remove_item_when_there_are_multiple_items_in_different_pages() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(2048), 0x34).unwrap();

        // Execute
        let result = target.remove(0);

        // Assert
        assert_that!(result, eq(true));

        let mut expected_indices_2 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_2[0] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(3));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(each(none())),
        );
        assert_that!(
            target.sparse_pages[1]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            none(),
        );
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_2)),
        );
        assert_that!(
            target.dense,
            elements_are![eq(&(TestHandle::new(2048), 0x34))]
        );
    }

    #[test]
    fn should_remove_item_and_swap_last_item_in_when_there_are_multiple_items_in_the_same_page() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(1), 0x34).unwrap();
        target.insert(TestHandle::new(2), 0x56).unwrap();

        // Execute
        let result = target.remove(0);

        // Assert
        assert_that!(result, eq(true));

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[1] = Some(1);
        expected_indices[2] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(2), 0x56)),
                eq(&(TestHandle::new(1), 0x34))
            ]
        );
    }

    #[test]
    fn should_remove_item_and_swap_last_item_in_when_there_are_multiple_items_in_different_pages() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(2048), 0x12).unwrap();
        target.insert(TestHandle::new(1), 0x34).unwrap();
        target.insert(TestHandle::new(2), 0x56).unwrap();

        // Execute
        let result = target.remove(2048);

        // Assert
        assert_that!(result, eq(true));

        let mut expected_indices_0 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_0[1] = Some(1);
        expected_indices_0[2] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(3));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_0)),
        );
        assert_that!(
            target.sparse_pages[1]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            none(),
        );
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(each(none())),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(2), 0x56)),
                eq(&(TestHandle::new(1), 0x34))
            ]
        );
    }

    #[test]
    fn should_return_false_when_removing_item_and_slot_is_already_empty() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();

        // Execute
        let result = target.remove(0);

        // Assert
        assert_that!(result, eq(false));

        assert_that!(target.sparse_pages.len(), eq(0));
        assert_that!(target.dense, is_empty());
    }

    // contains

    #[test]
    fn should_return_false_when_contains_called_and_slot_is_empty() {
        // Setup
        let target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();

        // Execute
        let result = target.contains(0);

        // Assert
        assert_that!(result, eq(false));

        assert_that!(target.sparse_pages.len(), eq(0));
        assert_that!(target.dense, is_empty());
    }

    #[test]
    fn should_return_true_when_contains_called_and_slot_is_filled() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();

        // Execute
        let result = target.contains(0);

        // Assert
        assert_that!(result, eq(true));

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(target.dense, elements_are![eq(&(TestHandle::new(0), 0x12))]);
    }

    // len

    #[test]
    fn should_return_length_when_no_items_exist() {
        // Setup
        let target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();

        // Execute
        let result = target.len();

        // Assert
        assert_that!(result, eq(0));

        assert_that!(target.sparse_pages.len(), eq(0));
        assert_that!(target.dense, is_empty());
    }

    #[test]
    fn should_return_length_when_there_is_only_one_item() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();

        // Execute
        let result = target.len();

        // Assert
        assert_that!(result, eq(1));

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(target.dense, elements_are![eq(&(TestHandle::new(0), 0x12))]);
    }

    #[test]
    fn should_return_length_when_there_is_multiple_items() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(1), 0x34).unwrap();
        target.insert(TestHandle::new(2048), 0x56).unwrap();

        // Execute
        let result = target.len();

        // Assert
        assert_that!(result, eq(3));

        let mut expected_indices_0 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_0[0] = Some(0);
        expected_indices_0[1] = Some(1);

        let mut expected_indices_2 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_2[0] = Some(2);

        assert_that!(target.sparse_pages.len(), eq(3));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_0)),
        );
        assert_that!(
            target.sparse_pages[1]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            none(),
        );
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_2)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(0), 0x12)),
                eq(&(TestHandle::new(1), 0x34)),
                eq(&(TestHandle::new(2048), 0x56))
            ]
        );
    }

    // get_item_handle_by_dense_index

    #[test]
    fn should_return_none_when_getting_item_handle_by_dense_index_and_no_items_exist() {
        // Setup
        let target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();

        // Execute
        let result = target.get_item_handle_by_dense_index(0);

        // Assert
        assert_that!(result, none());

        assert_that!(target.sparse_pages.len(), eq(0));
        assert_that!(target.dense, is_empty());
    }

    #[test]
    fn should_return_item_handle_when_getting_item_handle_by_dense_index_and_there_is_only_one_item()
     {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();

        // Execute
        let result = target.get_item_handle_by_dense_index(0);

        // Assert
        assert_that!(result, some(eq(TestHandle::new(0))));

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(target.dense, elements_are![eq(&(TestHandle::new(0), 0x12))]);
    }

    #[test]
    fn should_return_item_handle_when_getting_item_handle_by_dense_index_and_there_is_multiple_items()
     {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(1), 0x34).unwrap();
        target.insert(TestHandle::new(2048), 0x56).unwrap();

        // Execute
        let result = target.get_item_handle_by_dense_index(2);

        // Assert
        assert_that!(result, some(eq(TestHandle::new(2048))));

        let mut expected_indices_0 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_0[0] = Some(0);
        expected_indices_0[1] = Some(1);

        let mut expected_indices_2 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_2[0] = Some(2);

        assert_that!(target.sparse_pages.len(), eq(3));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_0)),
        );
        assert_that!(
            target.sparse_pages[1]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            none(),
        );
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_2)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(0), 0x12)),
                eq(&(TestHandle::new(1), 0x34)),
                eq(&(TestHandle::new(2048), 0x56))
            ]
        );
    }

    // insert

    #[test]
    fn should_insert_item_when_sparse_index_is_zero() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();

        // Execute
        target.insert(TestHandle::new(0), 0x12).unwrap();

        // Assert
        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(target.dense, elements_are![eq(&(TestHandle::new(0), 0x12))]);
    }

    #[test]
    fn should_insert_item_when_sparse_index_is_zero_and_same_page_already_contains_items() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(1), 0x12).unwrap();

        // Execute
        target.insert(TestHandle::new(0), 0x34).unwrap();

        // Assert
        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(1);
        expected_indices[1] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(1), 0x12)),
                eq(&(TestHandle::new(0), 0x34))
            ]
        );
    }

    #[test]
    fn should_insert_item_when_sparse_index_is_zero_and_different_page_already_contains_items() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(2048), 0x12).unwrap();

        // Execute
        target.insert(TestHandle::new(0), 0x34).unwrap();

        // Assert
        let mut expected_indices_0 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_0[0] = Some(1);

        let mut expected_indices_2 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_2[0] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(3));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_0)),
        );
        assert_that!(
            target.sparse_pages[1]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            none(),
        );
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_2)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(2048), 0x12)),
                eq(&(TestHandle::new(0), 0x34)),
            ]
        );
    }

    #[test]
    fn should_insert_item_when_sparse_index_is_high() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();

        // Execute
        target
            .insert(TestHandle::new(PAGE_SIZE_FOR_TESTS as u32 * 10), 0x12)
            .unwrap();

        // Assert
        let mut expected_indices_10 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_10[0] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(11));
        for i in 0..10 {
            assert_that!(
                target.sparse_pages[i]
                    .as_ref()
                    .map(|sparse_page| sparse_page.indices.as_ref()),
                none(),
            );
        }
        assert_that!(
            target.sparse_pages[10]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_10)),
        );
        assert_that!(
            target.dense,
            elements_are![eq(&(
                TestHandle::new(PAGE_SIZE_FOR_TESTS as u32 * 10),
                0x12
            ))]
        );
    }

    #[test]
    fn should_insert_item_when_sparse_index_is_high_and_same_page_already_contains_items() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target
            .insert(TestHandle::new(PAGE_SIZE_FOR_TESTS as u32 * 10 + 1), 0x12)
            .unwrap();

        // Execute
        target
            .insert(TestHandle::new(PAGE_SIZE_FOR_TESTS as u32 * 10), 0x34)
            .unwrap();

        // Assert
        let mut expected_indices_10 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_10[0] = Some(1);
        expected_indices_10[1] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(11));
        for i in 0..10 {
            assert_that!(
                target.sparse_pages[i]
                    .as_ref()
                    .map(|sparse_page| sparse_page.indices.as_ref()),
                none(),
            );
        }
        assert_that!(
            target.sparse_pages[10]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_10)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(PAGE_SIZE_FOR_TESTS as u32 * 10 + 1), 0x12)),
                eq(&(TestHandle::new(PAGE_SIZE_FOR_TESTS as u32 * 10), 0x34))
            ]
        );
    }

    #[test]
    fn should_insert_item_when_sparse_index_is_high_and_different_page_already_contains_items() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(2048), 0x12).unwrap();

        // Execute
        target
            .insert(TestHandle::new(PAGE_SIZE_FOR_TESTS as u32 * 10), 0x34)
            .unwrap();

        // Assert
        let mut expected_indices_2 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_2[0] = Some(0);

        let mut expected_indices_10 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_10[0] = Some(1);

        assert_that!(target.sparse_pages.len(), eq(11));
        for i in 0..2 {
            assert_that!(
                target.sparse_pages[i]
                    .as_ref()
                    .map(|sparse_page| sparse_page.indices.as_ref()),
                none(),
            );
        }
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_2)),
        );
        for i in 3..10 {
            assert_that!(
                target.sparse_pages[i]
                    .as_ref()
                    .map(|sparse_page| sparse_page.indices.as_ref()),
                none(),
            );
        }
        assert_that!(
            target.sparse_pages[10]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_10)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(2048), 0x12)),
                eq(&(TestHandle::new(PAGE_SIZE_FOR_TESTS as u32 * 10), 0x34))
            ]
        );
    }

    #[test]
    fn should_return_slot_full_when_inserting_item_and_slot_is_already_filled() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();

        // Execute
        let result = target.insert(TestHandle::new(0), 0x34);

        // Assert
        assert_that!(
            result,
            err(matches_pattern!(NeuclidioError::EcsError(
                matches_pattern!(EcsError::PagedSparseSetError(matches_pattern!(
                    PagedSparseSetError::SlotFull
                )))
            )))
        );

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(target.dense, elements_are![eq(&(TestHandle::new(0), 0x12))]);
    }

    // take

    #[test]
    fn should_take_item_when_there_is_only_one_item() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();

        // Execute
        let result = target.take(0);

        // Assert
        assert_that!(result, some(eq((TestHandle::new(0), 0x12))));

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(each(none())),
        );
        assert_that!(target.dense, is_empty());
    }

    #[test]
    fn should_take_item_when_there_are_multiple_items_in_the_same_page() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(1), 0x34).unwrap();

        // Execute
        let result = target.take(0);

        // Assert
        assert_that!(result, some(eq((TestHandle::new(0), 0x12))));

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[1] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(target.dense, elements_are![eq(&(TestHandle::new(1), 0x34))]);
    }

    #[test]
    fn should_take_item_when_there_are_multiple_items_in_different_pages() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(2048), 0x34).unwrap();

        // Execute
        let result = target.take(0);

        // Assert
        assert_that!(result, some(eq((TestHandle::new(0), 0x12))));

        let mut expected_indices_2 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_2[0] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(3));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(each(none())),
        );
        assert_that!(
            target.sparse_pages[1]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            none(),
        );
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_2)),
        );
        assert_that!(
            target.dense,
            elements_are![eq(&(TestHandle::new(2048), 0x34))]
        );
    }

    #[test]
    fn should_take_item_and_swap_last_item_in_when_there_are_multiple_items_in_the_same_page() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(1), 0x34).unwrap();
        target.insert(TestHandle::new(2), 0x56).unwrap();

        // Execute
        let result = target.take(0);

        // Assert
        assert_that!(result, some(eq((TestHandle::new(0), 0x12))));

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[1] = Some(1);
        expected_indices[2] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(2), 0x56)),
                eq(&(TestHandle::new(1), 0x34))
            ]
        );
    }

    #[test]
    fn should_take_item_and_swap_last_item_in_when_there_are_multiple_items_in_different_pages() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(2048), 0x12).unwrap();
        target.insert(TestHandle::new(1), 0x34).unwrap();
        target.insert(TestHandle::new(2), 0x56).unwrap();

        // Execute
        let result = target.take(2048);

        // Assert
        assert_that!(result, some(eq((TestHandle::new(2048), 0x12))));

        let mut expected_indices_0 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_0[1] = Some(1);
        expected_indices_0[2] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(3));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_0)),
        );
        assert_that!(
            target.sparse_pages[1]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            none(),
        );
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(each(none())),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(2), 0x56)),
                eq(&(TestHandle::new(1), 0x34))
            ]
        );
    }

    #[test]
    fn should_return_none_when_removing_item_and_slot_is_already_empty() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();

        // Execute
        let result = target.take(0);

        // Assert
        assert_that!(result, none());

        assert_that!(target.sparse_pages.len(), eq(0));
        assert_that!(target.dense, is_empty());
    }

    // iter

    #[test]
    fn should_iterate_over_dense_entries_when_there_is_no_items() {
        // Setup
        let target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();

        // Execute
        let mut result = target.iter();

        // Assert
        assert_that!(result.next(), none());

        assert_that!(target.sparse_pages.len(), eq(0));
        assert_that!(target.dense, is_empty());
    }

    #[test]
    fn should_iterate_over_dense_entries_when_there_is_many_items() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(1), 0x34).unwrap();
        target.insert(TestHandle::new(2), 0x56).unwrap();
        target.insert(TestHandle::new(3), 0x78).unwrap();
        target.insert(TestHandle::new(4), 0x90).unwrap();

        // Execute
        let mut result = target.iter();

        // Assert
        assert_that!(result.next(), some(eq((TestHandle::new(0), &0x12))));
        assert_that!(result.next(), some(eq((TestHandle::new(1), &0x34))));
        assert_that!(result.next(), some(eq((TestHandle::new(2), &0x56))));
        assert_that!(result.next(), some(eq((TestHandle::new(3), &0x78))));
        assert_that!(result.next(), some(eq((TestHandle::new(4), &0x90))));

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);
        expected_indices[1] = Some(1);
        expected_indices[2] = Some(2);
        expected_indices[3] = Some(3);
        expected_indices[4] = Some(4);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(0), 0x12)),
                eq(&(TestHandle::new(1), 0x34)),
                eq(&(TestHandle::new(2), 0x56)),
                eq(&(TestHandle::new(3), 0x78)),
                eq(&(TestHandle::new(4), 0x90)),
            ]
        );
    }

    #[test]
    fn should_iterate_over_dense_entries_when_there_is_many_items_and_complex_state() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(6), 0xff).unwrap();
        target.insert(TestHandle::new(2), 0x56).unwrap();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(4), 0x90).unwrap();
        target.insert(TestHandle::new(1), 0x34).unwrap();
        target.insert(TestHandle::new(3), 0x78).unwrap();
        target.insert(TestHandle::new(5), 0xff).unwrap();
        target.remove(6);
        target.remove(5);

        // Execute
        let mut result = target.iter();

        // Assert
        assert_that!(result.next(), some(eq((TestHandle::new(3), &0x78))));
        assert_that!(result.next(), some(eq((TestHandle::new(2), &0x56))));
        assert_that!(result.next(), some(eq((TestHandle::new(0), &0x12))));
        assert_that!(result.next(), some(eq((TestHandle::new(4), &0x90))));
        assert_that!(result.next(), some(eq((TestHandle::new(1), &0x34))));

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(2);
        expected_indices[1] = Some(4);
        expected_indices[2] = Some(1);
        expected_indices[3] = Some(0);
        expected_indices[4] = Some(3);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(3), 0x78)),
                eq(&(TestHandle::new(2), 0x56)),
                eq(&(TestHandle::new(0), 0x12)),
                eq(&(TestHandle::new(4), 0x90)),
                eq(&(TestHandle::new(1), 0x34)),
            ]
        );
    }

    // iter_mut

    #[test]
    fn should_mutably_iterate_over_dense_entries_when_there_is_no_items() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();

        // Execute
        let mut result = target.iter_mut();

        // Assert
        assert_that!(result.next(), none());

        drop(result);

        assert_that!(target.sparse_pages.len(), eq(0));
        assert_that!(target.dense, is_empty());
    }

    #[test]
    fn should_mutably_iterate_over_dense_entries_when_there_is_many_items() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(1), 0x34).unwrap();
        target.insert(TestHandle::new(2), 0x56).unwrap();
        target.insert(TestHandle::new(3), 0x78).unwrap();
        target.insert(TestHandle::new(4), 0x90).unwrap();

        // Execute
        let mut result = target.iter_mut();
        *result.next().unwrap().1 = 0xff;
        *result.next().unwrap().1 = 0xff;
        *result.next().unwrap().1 = 0xff;
        *result.next().unwrap().1 = 0xff;
        *result.next().unwrap().1 = 0xff;

        drop(result);

        // Assert
        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);
        expected_indices[1] = Some(1);
        expected_indices[2] = Some(2);
        expected_indices[3] = Some(3);
        expected_indices[4] = Some(4);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(0), 0xff)),
                eq(&(TestHandle::new(1), 0xff)),
                eq(&(TestHandle::new(2), 0xff)),
                eq(&(TestHandle::new(3), 0xff)),
                eq(&(TestHandle::new(4), 0xff)),
            ]
        );
    }

    #[test]
    fn should_mutably_iterate_over_dense_entries_when_there_is_many_items_and_complex_state() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(6), 0xff).unwrap();
        target.insert(TestHandle::new(2), 0x56).unwrap();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(4), 0x90).unwrap();
        target.insert(TestHandle::new(1), 0x34).unwrap();
        target.insert(TestHandle::new(3), 0x78).unwrap();
        target.insert(TestHandle::new(5), 0xff).unwrap();
        target.remove(6);
        target.remove(5);

        // Execute
        let mut result = target.iter_mut();
        *result.next().unwrap().1 = 0xff;
        *result.next().unwrap().1 = 0xff;
        *result.next().unwrap().1 = 0xff;
        *result.next().unwrap().1 = 0xff;
        *result.next().unwrap().1 = 0xff;

        drop(result);

        // Assert
        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(2);
        expected_indices[1] = Some(4);
        expected_indices[2] = Some(1);
        expected_indices[3] = Some(0);
        expected_indices[4] = Some(3);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(3), 0xff)),
                eq(&(TestHandle::new(2), 0xff)),
                eq(&(TestHandle::new(0), 0xff)),
                eq(&(TestHandle::new(4), 0xff)),
                eq(&(TestHandle::new(1), 0xff)),
            ]
        );
    }

    // get

    #[test]
    fn should_get_item_when_there_is_only_one_item() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();

        // Execute
        let result = target.get(0);

        // Assert
        assert_that!(result, some(points_to(eq(0x12))),);

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(target.dense, elements_are![eq(&(TestHandle::new(0), 0x12))]);
    }

    #[test]
    fn should_get_item_when_there_is_multiple_items_in_the_same_page() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(2), 0x56).unwrap();

        // Execute
        let result = target.get(0);

        // Assert
        assert_that!(result, some(points_to(eq(0x12))),);

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);
        expected_indices[2] = Some(1);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(0), 0x12)),
                eq(&(TestHandle::new(2), 0x56))
            ]
        );
    }

    #[test]
    fn should_get_item_when_there_is_multiple_items_in_different_pages() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(2048), 0x56).unwrap();

        // Execute
        let result = target.get(0);

        // Assert
        assert_that!(result, some(points_to(eq(0x12))),);

        let mut expected_indices_0 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_0[0] = Some(0);

        let mut expected_indices_2 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_2[0] = Some(1);

        assert_that!(target.sparse_pages.len(), eq(3));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_0)),
        );
        assert_that!(
            target.sparse_pages[1]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            none(),
        );
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_2)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(0), 0x12)),
                eq(&(TestHandle::new(2048), 0x56))
            ]
        );
    }

    #[test]
    fn should_return_none_when_getting_item_and_slot_is_empty() {
        // Setup
        let target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();

        // Execute
        let result = target.get(0);

        // Assert
        assert_that!(result, none());

        assert_that!(target.sparse_pages.len(), eq(0));
        assert_that!(target.dense, is_empty());
    }

    // get_mut

    #[test]
    fn should_get_mutable_item_when_there_is_only_one_item() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();

        // Execute
        let result = target.get_mut(0).unwrap();
        *result = 0x34;

        // Assert
        assert_that!(result, derefs_to(eq(&0x34)),);

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(target.dense, elements_are![eq(&(TestHandle::new(0), 0x34))]);
    }

    #[test]
    fn should_get_mutable_item_when_there_is_multiple_items_in_the_same_page() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(2), 0x56).unwrap();

        // Execute
        let result = target.get_mut(0).unwrap();
        *result = 0x34;

        // Assert
        assert_that!(result, derefs_to(eq(&0x34)),);

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);
        expected_indices[2] = Some(1);

        assert_that!(target.sparse_pages.len(), eq(1));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(0), 0x34)),
                eq(&(TestHandle::new(2), 0x56))
            ]
        );
    }

    #[test]
    fn should_get_mutable_item_when_there_is_multiple_items_in_different_pages() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();
        target.insert(TestHandle::new(0), 0x12).unwrap();
        target.insert(TestHandle::new(2048), 0x56).unwrap();

        // Execute
        let result = target.get_mut(0).unwrap();
        *result = 0x34;

        // Assert
        assert_that!(result, derefs_to(eq(&0x34)),);

        let mut expected_indices_0 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_0[0] = Some(0);

        let mut expected_indices_2 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_2[0] = Some(1);

        assert_that!(target.sparse_pages.len(), eq(3));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_0)),
        );
        assert_that!(
            target.sparse_pages[1]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            none(),
        );
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page| sparse_page.indices.as_ref()),
            some(eq(&expected_indices_2)),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(TestHandle::new(0), 0x34)),
                eq(&(TestHandle::new(2048), 0x56))
            ]
        );
    }

    #[test]
    fn should_return_none_when_getting_mutable_item_and_slot_is_empty() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, TestHandle, u8>::new();

        // Execute
        let result = target.get_mut(0);

        // Assert
        assert_that!(result, none());

        assert_that!(target.sparse_pages.len(), eq(0));
        assert_that!(target.dense, is_empty());
    }
}
