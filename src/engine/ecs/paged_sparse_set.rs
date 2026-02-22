use crate::error::NeuclidioResult;

#[derive(Debug)]
pub enum PagedSparseSetError {
    SlotFull,
}

#[derive(Debug)]
pub struct PagedSparseSet<const PAGE_SIZE: usize, T> {
    sparse_pages: Vec<Option<Box<SparseSetPage<PAGE_SIZE>>>>,
    dense: Vec<(u32, T)>,
}

#[derive(Debug)]
pub struct SparseSetPage<const PAGE_SIZE: usize> {
    indices: [Option<usize>; PAGE_SIZE],
}

impl<const PAGE_SIZE: usize, T> PagedSparseSet<PAGE_SIZE, T> {
    const PAGE_INDEX_BITSHIFT: usize = PAGE_SIZE.trailing_zeros() as usize;
    const ITEM_INDEX_BITMASK: usize = PAGE_SIZE - 1;

    pub fn new() -> Self {
        Self {
            sparse_pages: vec![],
            dense: vec![],
        }
    }

    // TODO: Implement a deallocation function which would remove unused pages

    pub fn insert(&mut self, sparse_index: u32, item: T) -> NeuclidioResult<()> {
        let (sparse_page_index, sparse_item_index) = Self::get_page_and_item_indices(sparse_index);

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
        self.dense.push((sparse_index, item));

        sparse_page.indices[sparse_item_index].replace(dense_index);
        Ok(())
    }

    pub fn remove(&mut self, sparse_index: u32) -> Option<T> {
        let (sparse_page_index, sparse_item_index) = Self::get_page_and_item_indices(sparse_index);
        let sparse_page = self.sparse_pages.get_mut(sparse_page_index)?.as_mut()?;
        let dense_index = sparse_page.indices[sparse_item_index].take()?;
        let (_, item) = self.dense.swap_remove(dense_index);

        if let Some((swapped_sparse_index, _)) = self.dense.get(dense_index) {
            let (swapped_sparse_page_index, swapped_sparse_item_index) =
                Self::get_page_and_item_indices(*swapped_sparse_index);

            let swapped_sparse_page = self.sparse_pages[swapped_sparse_page_index]
                .as_mut()
                .unwrap();

            swapped_sparse_page.indices[swapped_sparse_item_index].replace(dense_index);
        }

        Some(item)
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        self.dense
            .iter()
            .map(|(sparse_index, item)| (*sparse_index, item))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (u32, &mut T)> {
        self.dense
            .iter_mut()
            .map(|(sparse_index, item)| (*sparse_index, item))
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

    pub fn contains(&self, sparse_index: u32) -> bool {
        let (sparse_page_index, sparse_item_index) = Self::get_page_and_item_indices(sparse_index);

        if let Some(Some(sparse_page)) = self.sparse_pages.get(sparse_page_index) {
            sparse_page.indices[sparse_item_index].is_some()
        } else {
            false
        }
    }

    fn get_page_and_item_indices(sparse_index: u32) -> (usize, usize) {
        let sparse_index = sparse_index as usize;
        let sparse_page_index = sparse_index >> Self::PAGE_INDEX_BITSHIFT;
        let sparse_item_index = sparse_index & Self::ITEM_INDEX_BITMASK;

        (sparse_page_index, sparse_item_index)
    }
}

impl<const PAGE_SIZE: usize> SparseSetPage<PAGE_SIZE> {
    const ASSERT_PAGE_SIZE_POWER_OF_TWO: () = assert!(
        PAGE_SIZE.is_power_of_two(),
        "PAGE_SIZE must be a power of two"
    );

    pub fn new() -> Self {
        let _ = Self::ASSERT_PAGE_SIZE_POWER_OF_TWO;

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

    // Insertion

    #[test]
    fn should_insert_item_when_sparse_index_is_zero() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();

        // Execute
        target.insert(0, 0x12).unwrap();

        // Assert
        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);

        assert_that!(target.sparse_pages, len(eq(1)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(target.dense, elements_are![eq(&(0, 0x12))]);
    }

    #[test]
    fn should_insert_item_when_sparse_index_is_zero_and_same_page_already_contains_items() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(1, 0x12).unwrap();

        // Execute
        target.insert(0, 0x34).unwrap();

        // Assert
        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(1);
        expected_indices[1] = Some(0);

        assert_that!(target.sparse_pages, len(eq(1)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(target.dense, elements_are![eq(&(1, 0x12)), eq(&(0, 0x34))]);
    }

    #[test]
    fn should_insert_item_when_sparse_index_is_zero_and_different_page_already_contains_items() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(2048, 0x12).unwrap();

        // Execute
        target.insert(0, 0x34).unwrap();

        // Assert
        let mut expected_indices_0 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_0[0] = Some(1);

        let mut expected_indices_2 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_2[0] = Some(0);

        assert_that!(target.sparse_pages, len(eq(3)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices_0),
            })),
        );
        assert_that!(
            target.sparse_pages[1]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            none(),
        );
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices_2),
            })),
        );
        assert_that!(
            target.dense,
            elements_are![eq(&(2048, 0x12)), eq(&(0, 0x34))]
        );
    }

    #[test]
    fn should_insert_item_when_sparse_index_is_high() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();

        // Execute
        target
            .insert(PAGE_SIZE_FOR_TESTS as u32 * 10, 0x12)
            .unwrap();

        // Assert
        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);

        assert_that!(target.sparse_pages, len(eq(11)));
        for i in 0..10 {
            assert_that!(
                target.sparse_pages[i]
                    .as_ref()
                    .map(|sparse_page_option| sparse_page_option.as_ref()),
                none(),
            );
        }
        assert_that!(
            target.sparse_pages[10]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(
            target.dense,
            elements_are![eq(&(PAGE_SIZE_FOR_TESTS as u32 * 10, 0x12))]
        );
    }

    #[test]
    fn should_insert_item_when_sparse_index_is_high_and_same_page_already_contains_items() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target
            .insert(PAGE_SIZE_FOR_TESTS as u32 * 10 + 1, 0x12)
            .unwrap();

        // Execute
        target
            .insert(PAGE_SIZE_FOR_TESTS as u32 * 10, 0x34)
            .unwrap();

        // Assert
        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(1);
        expected_indices[1] = Some(0);

        assert_that!(target.sparse_pages, len(eq(11)));
        for i in 0..10 {
            assert_that!(
                target.sparse_pages[i]
                    .as_ref()
                    .map(|sparse_page_option| sparse_page_option.as_ref()),
                none(),
            );
        }
        assert_that!(
            target.sparse_pages[10]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(PAGE_SIZE_FOR_TESTS as u32 * 10 + 1, 0x12)),
                eq(&(PAGE_SIZE_FOR_TESTS as u32 * 10, 0x34))
            ]
        );
    }

    #[test]
    fn should_insert_item_when_sparse_index_is_high_and_different_page_already_contains_items() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(2048, 0x12).unwrap();

        // Execute
        target
            .insert(PAGE_SIZE_FOR_TESTS as u32 * 10, 0x34)
            .unwrap();

        // Assert
        let mut expected_indices_10 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_10[0] = Some(1);

        let mut expected_indices_2 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_2[0] = Some(0);

        assert_that!(target.sparse_pages, len(eq(11)));
        for i in 0..2 {
            assert_that!(
                target.sparse_pages[i]
                    .as_ref()
                    .map(|sparse_page_option| sparse_page_option.as_ref()),
                none(),
            );
        }
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices_2),
            })),
        );
        for i in 3..10 {
            assert_that!(
                target.sparse_pages[i]
                    .as_ref()
                    .map(|sparse_page_option| sparse_page_option.as_ref()),
                none(),
            );
        }
        assert_that!(
            target.sparse_pages[10]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices_10),
            })),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(2048, 0x12)),
                eq(&(PAGE_SIZE_FOR_TESTS as u32 * 10, 0x34))
            ]
        );
    }

    #[test]
    fn should_return_slot_full_when_inserting_item_and_slot_is_already_filled() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(0, 0x12).unwrap();

        // Execute
        let result = target.insert(0, 0x34);

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

        assert_that!(target.sparse_pages, len(eq(1)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(target.dense, elements_are![eq(&(0, 0x12))]);
    }

    // Removal

    #[test]
    fn should_remove_item_when_there_is_only_one_item() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(0, 0x12).unwrap();

        // Execute
        let result = target.remove(0);

        // Assert
        assert_that!(result, some(eq(0x12)));

        assert_that!(target.sparse_pages, len(eq(1)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: each(none()),
            })),
        );
        assert_that!(target.dense, is_empty());
    }

    #[test]
    fn should_remove_item_when_there_are_multiple_items_in_the_same_page() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(0, 0x12).unwrap();
        target.insert(1, 0x34).unwrap();

        // Execute
        let result = target.remove(0);

        // Assert
        assert_that!(result, some(eq(0x12)));

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[1] = Some(0);

        assert_that!(target.sparse_pages, len(eq(1)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(target.dense, elements_are![eq(&(1, 0x34))]);
    }

    #[test]
    fn should_remove_item_when_there_are_multiple_items_in_different_pages() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(0, 0x12).unwrap();
        target.insert(2048, 0x34).unwrap();

        // Execute
        let result = target.remove(0);

        // Assert
        assert_that!(result, some(eq(0x12)));

        let mut expected_indices_2 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_2[0] = Some(0);

        assert_that!(target.sparse_pages, len(eq(3)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: each(none()),
            })),
        );
        assert_that!(
            target.sparse_pages[1]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            none(),
        );
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices_2),
            })),
        );
        assert_that!(target.dense, elements_are![eq(&(2048, 0x34))]);
    }

    #[test]
    fn should_remove_item_and_swap_last_item_in_when_there_are_multiple_items_in_the_same_page() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(0, 0x12).unwrap();
        target.insert(1, 0x34).unwrap();
        target.insert(2, 0x56).unwrap();

        // Execute
        let result = target.remove(0);

        // Assert
        assert_that!(result, some(eq(0x12)));

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[1] = Some(1);
        expected_indices[2] = Some(0);

        assert_that!(target.sparse_pages, len(eq(1)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(target.dense, elements_are![eq(&(2, 0x56)), eq(&(1, 0x34))]);
    }

    #[test]
    fn should_remove_item_and_swap_last_item_in_when_there_are_multiple_items_in_different_pages() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(2048, 0x12).unwrap();
        target.insert(1, 0x34).unwrap();
        target.insert(2, 0x56).unwrap();

        // Execute
        let result = target.remove(2048);

        // Assert
        assert_that!(result, some(eq(0x12)));

        let mut expected_indices_0 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_0[1] = Some(1);
        expected_indices_0[2] = Some(0);

        assert_that!(target.sparse_pages, len(eq(3)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices_0),
            })),
        );
        assert_that!(
            target.sparse_pages[1]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            none(),
        );
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: each(none()),
            })),
        );
        assert_that!(target.dense, elements_are![eq(&(2, 0x56)), eq(&(1, 0x34))]);
    }

    #[test]
    fn should_return_none_when_deleting_item_and_slot_is_already_empty() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();

        // Execute
        let result = target.remove(0);

        // Assert
        assert_that!(result, none(),);

        assert_that!(target.sparse_pages, is_empty());
        assert_that!(target.dense, is_empty());
    }

    // Reference retrieval

    #[test]
    fn should_get_item_when_there_is_only_one_item() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(0, 0x12).unwrap();

        // Execute
        let result = target.get(0);

        // Assert
        assert_that!(result, some(points_to(eq(0x12))),);

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);

        assert_that!(target.sparse_pages, len(eq(1)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(target.dense, elements_are![eq(&(0, 0x12))]);
    }

    #[test]
    fn should_get_item_when_there_is_multiple_items_in_the_same_page() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(0, 0x12).unwrap();
        target.insert(2, 0x56).unwrap();

        // Execute
        let result = target.get(0);

        // Assert
        assert_that!(result, some(points_to(eq(0x12))),);

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);
        expected_indices[2] = Some(1);

        assert_that!(target.sparse_pages, len(eq(1)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(target.dense, elements_are![eq(&(0, 0x12)), eq(&(2, 0x56))]);
    }

    #[test]
    fn should_get_item_when_there_is_multiple_items_in_different_pages() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(0, 0x12).unwrap();
        target.insert(2048, 0x56).unwrap();

        // Execute
        let result = target.get(0);

        // Assert
        assert_that!(result, some(points_to(eq(0x12))),);

        let mut expected_indices_0 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_0[0] = Some(0);

        let mut expected_indices_2 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_2[0] = Some(1);

        assert_that!(target.sparse_pages, len(eq(3)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices_0),
            })),
        );
        assert_that!(
            target.sparse_pages[1]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            none(),
        );
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices_2),
            })),
        );
        assert_that!(
            target.dense,
            elements_are![eq(&(0, 0x12)), eq(&(2048, 0x56))]
        );
    }

    #[test]
    fn should_return_none_when_getting_item_and_slot_is_empty() {
        // Setup
        let target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();

        // Execute
        let result = target.get(0);

        // Assert
        assert_that!(result, none(),);

        assert_that!(target.sparse_pages, is_empty());
        assert_that!(target.dense, is_empty());
    }

    // Mutable reference retrieval

    #[test]
    fn should_get_mutable_item_when_there_is_only_one_item() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(0, 0x12).unwrap();

        // Execute
        let result = target.get_mut(0).unwrap();
        *result = 0x34;

        // Assert
        assert_that!(result, derefs_to(eq(&0x34)),);

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);

        assert_that!(target.sparse_pages, len(eq(1)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(target.dense, elements_are![eq(&(0, 0x34))]);
    }

    #[test]
    fn should_get_mutable_item_when_there_is_multiple_items_in_the_same_page() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(0, 0x12).unwrap();
        target.insert(2, 0x56).unwrap();

        // Execute
        let result = target.get_mut(0).unwrap();
        *result = 0x34;

        // Assert
        assert_that!(result, derefs_to(eq(&0x34)),);

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);
        expected_indices[2] = Some(1);

        assert_that!(target.sparse_pages, len(eq(1)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(target.dense, elements_are![eq(&(0, 0x34)), eq(&(2, 0x56))]);
    }

    #[test]
    fn should_get_mutable_item_when_there_is_multiple_items_in_different_pages() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(0, 0x12).unwrap();
        target.insert(2048, 0x56).unwrap();

        // Execute
        let result = target.get_mut(0).unwrap();
        *result = 0x34;

        // Assert
        assert_that!(result, derefs_to(eq(&0x34)),);

        let mut expected_indices_0 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_0[0] = Some(0);

        let mut expected_indices_2 = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices_2[0] = Some(1);

        assert_that!(target.sparse_pages, len(eq(3)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices_0),
            })),
        );
        assert_that!(
            target.sparse_pages[1]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            none(),
        );
        assert_that!(
            target.sparse_pages[2]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices_2),
            })),
        );
        assert_that!(
            target.dense,
            elements_are![eq(&(0, 0x34)), eq(&(2048, 0x56))]
        );
    }

    #[test]
    fn should_return_none_when_getting_mutable_item_and_slot_is_empty() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();

        // Execute
        let result = target.get_mut(0);

        // Assert
        assert_that!(result, none(),);

        assert_that!(target.sparse_pages, is_empty());
        assert_that!(target.dense, is_empty());
    }

    // Contains check

    #[test]
    fn should_return_false_when_contains_called_and_slot_is_empty() {
        // Setup
        let target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();

        // Execute
        let result = target.contains(0);

        // Assert
        assert_that!(result, eq(false));

        assert_that!(target.sparse_pages, is_empty());
        assert_that!(target.dense, is_empty());
    }

    #[test]
    fn should_return_true_when_contains_called_and_slot_is_filled() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(0, 0x12).unwrap();

        // Execute
        let result = target.contains(0);

        // Assert
        assert_that!(result, eq(true));

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);

        assert_that!(target.sparse_pages, len(eq(1)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(target.dense, elements_are![eq(&(0, 0x12))]);
    }

    // Iteration

    #[test]
    fn should_iterate_over_dense_entries_when_there_is_no_items() {
        // Setup
        let target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();

        // Execute
        let mut result = target.iter();

        // Assert
        assert_that!(result.next(), none());

        assert_that!(target.sparse_pages, is_empty());
        assert_that!(target.dense, is_empty());
    }

    #[test]
    fn should_iterate_over_dense_entries_when_there_is_many_items() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(0, 0x12).unwrap();
        target.insert(1, 0x34).unwrap();
        target.insert(2, 0x56).unwrap();
        target.insert(3, 0x78).unwrap();
        target.insert(4, 0x90).unwrap();

        // Execute
        let mut result = target.iter();

        // Assert
        assert_that!(result.next(), some(eq((0, &0x12))));
        assert_that!(result.next(), some(eq((1, &0x34))));
        assert_that!(result.next(), some(eq((2, &0x56))));
        assert_that!(result.next(), some(eq((3, &0x78))));
        assert_that!(result.next(), some(eq((4, &0x90))));

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(0);
        expected_indices[1] = Some(1);
        expected_indices[2] = Some(2);
        expected_indices[3] = Some(3);
        expected_indices[4] = Some(4);

        assert_that!(target.sparse_pages, len(eq(1)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(0, 0x12)),
                eq(&(1, 0x34)),
                eq(&(2, 0x56)),
                eq(&(3, 0x78)),
                eq(&(4, 0x90)),
            ]
        );
    }

    #[test]
    fn should_iterate_over_dense_entries_when_there_is_many_items_and_complex_state() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(6, 0xff).unwrap();
        target.insert(2, 0x56).unwrap();
        target.insert(0, 0x12).unwrap();
        target.insert(4, 0x90).unwrap();
        target.insert(1, 0x34).unwrap();
        target.insert(3, 0x78).unwrap();
        target.insert(5, 0xff).unwrap();
        target.remove(6).unwrap();
        target.remove(5).unwrap();

        // Execute
        let mut result = target.iter();

        // Assert
        assert_that!(result.next(), some(eq((3, &0x78))));
        assert_that!(result.next(), some(eq((2, &0x56))));
        assert_that!(result.next(), some(eq((0, &0x12))));
        assert_that!(result.next(), some(eq((4, &0x90))));
        assert_that!(result.next(), some(eq((1, &0x34))));

        let mut expected_indices = [None; PAGE_SIZE_FOR_TESTS];
        expected_indices[0] = Some(2);
        expected_indices[1] = Some(4);
        expected_indices[2] = Some(1);
        expected_indices[3] = Some(0);
        expected_indices[4] = Some(3);

        assert_that!(target.sparse_pages, len(eq(1)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(3, 0x78)),
                eq(&(2, 0x56)),
                eq(&(0, 0x12)),
                eq(&(4, 0x90)),
                eq(&(1, 0x34)),
            ]
        );
    }

    // Mutable iteration

    #[test]
    fn should_mutably_iterate_over_dense_entries_when_there_is_no_items() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();

        // Execute
        let mut result = target.iter_mut();

        // Assert
        assert_that!(result.next(), none());

        drop(result);

        assert_that!(target.sparse_pages, is_empty());
        assert_that!(target.dense, is_empty());
    }

    #[test]
    fn should_mutably_iterate_over_dense_entries_when_there_is_many_items() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(0, 0x12).unwrap();
        target.insert(1, 0x34).unwrap();
        target.insert(2, 0x56).unwrap();
        target.insert(3, 0x78).unwrap();
        target.insert(4, 0x90).unwrap();

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

        assert_that!(target.sparse_pages, len(eq(1)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(0, 0xff)),
                eq(&(1, 0xff)),
                eq(&(2, 0xff)),
                eq(&(3, 0xff)),
                eq(&(4, 0xff)),
            ]
        );
    }

    #[test]
    fn should_mutably_iterate_over_dense_entries_when_there_is_many_items_and_complex_state() {
        // Setup
        let mut target = PagedSparseSet::<PAGE_SIZE_FOR_TESTS, u8>::new();
        target.insert(6, 0xff).unwrap();
        target.insert(2, 0x56).unwrap();
        target.insert(0, 0x12).unwrap();
        target.insert(4, 0x90).unwrap();
        target.insert(1, 0x34).unwrap();
        target.insert(3, 0x78).unwrap();
        target.insert(5, 0xff).unwrap();
        target.remove(6).unwrap();
        target.remove(5).unwrap();

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

        assert_that!(target.sparse_pages, len(eq(1)));
        assert_that!(
            target.sparse_pages[0]
                .as_ref()
                .map(|sparse_page_option| sparse_page_option.as_ref()),
            some(matches_pattern!(SparseSetPage::<PAGE_SIZE_FOR_TESTS> {
                indices: eq(&expected_indices),
            })),
        );
        assert_that!(
            target.dense,
            elements_are![
                eq(&(3, 0xff)),
                eq(&(2, 0xff)),
                eq(&(0, 0xff)),
                eq(&(4, 0xff)),
                eq(&(1, 0xff)),
            ]
        );
    }
}
