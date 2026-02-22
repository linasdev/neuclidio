use crate::engine::ecs::entity::Entity;

#[derive(Debug)]
pub struct EntityPool {
    generations: Vec<u32>,
    free_sparse_indices: Vec<u32>,
    active_sparse_indices: Vec<u32>,
    active_sparse_index_locations: Vec<Option<usize>>,
}

impl EntityPool {
    pub fn new() -> Self {
        Self {
            generations: vec![],
            free_sparse_indices: vec![],
            active_sparse_indices: vec![],
            active_sparse_index_locations: vec![],
        }
    }

    pub fn add_entity(&mut self) -> Entity {
        let sparse_index = if let Some(sparse_index) = self.free_sparse_indices.pop() {
            sparse_index
        } else {
            let sparse_index = self.generations.len() as u32;
            self.generations.push(0);
            self.active_sparse_index_locations.push(None);
            sparse_index
        };

        let active_sparse_index_location = self.active_sparse_indices.len();
        self.active_sparse_indices.push(sparse_index);
        self.active_sparse_index_locations[sparse_index as usize] =
            Some(active_sparse_index_location);

        Entity {
            sparse_index,
            generation: self.generations[sparse_index as usize],
        }
    }

    pub fn remove_entity(&mut self, entity: Entity) -> bool {
        let generation =
            if let Some(generation) = self.generations.get_mut(entity.sparse_index as usize) {
                generation
            } else {
                return false;
            };

        if *generation != entity.generation {
            return false;
        }

        let active_sparse_index_location_option = if let Some(active_sparse_index_location_option) =
            self.active_sparse_index_locations
                .get_mut(entity.sparse_index as usize)
        {
            active_sparse_index_location_option
        } else {
            return false;
        };

        let active_sparse_index_location =
            if let Some(active_sparse_index_location) = active_sparse_index_location_option {
                *active_sparse_index_location
            } else {
                return false;
            };

        *generation += 1;
        *active_sparse_index_location_option = None;
        self.free_sparse_indices.push(entity.sparse_index);

        let _ = self
            .active_sparse_indices
            .swap_remove(active_sparse_index_location);

        if let Some(swapped_active_sparse_index) = self
            .active_sparse_indices
            .get_mut(active_sparse_index_location)
        {
            self.active_sparse_index_locations[*swapped_active_sparse_index as usize]
                .replace(active_sparse_index_location);
        }

        true
    }

    pub fn clear(&mut self) {
        self.generations.clear();
        self.free_sparse_indices.clear();
        self.active_sparse_indices.clear();
        self.active_sparse_index_locations.clear();
    }

    pub fn active_entities(&self) -> impl Iterator<Item = Entity> {
        self.active_sparse_indices
            .iter()
            .map(|&sparse_index| Entity {
                sparse_index,
                generation: self.generations[sparse_index as usize],
            })
    }

    pub fn is_entity_active(&self, entity: Entity) -> bool {
        let generation =
            if let Some(generation) = self.generations.get(entity.sparse_index as usize) {
                generation
            } else {
                return false;
            };

        *generation == entity.generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;

    // Entity addition

    #[test]
    fn should_add_entity_when_no_free_sparse_index_exists() {
        // Setup
        let mut target = EntityPool::new();

        // Execute
        let result = target.add_entity();

        // Assert
        assert_that!(
            result,
            matches_pattern!(Entity {
                sparse_index: 0,
                generation: 0,
            })
        );

        assert_that!(
            target,
            matches_pattern!(EntityPool {
                generations: elements_are![eq(&0)],
                free_sparse_indices: is_empty(),
                active_sparse_indices: elements_are![eq(&0)],
                active_sparse_index_locations: elements_are![some(eq(&0))],
            })
        );
    }

    #[test]
    fn should_add_multiple_entities_when_no_free_sparse_index_exists() {
        // Setup
        let mut target = EntityPool::new();

        // Execute
        let result1 = target.add_entity();
        let result2 = target.add_entity();
        let result3 = target.add_entity();

        // Assert
        assert_that!(
            result1,
            matches_pattern!(Entity {
                sparse_index: 0,
                generation: 0,
            })
        );
        assert_that!(
            result2,
            matches_pattern!(Entity {
                sparse_index: 1,
                generation: 0,
            })
        );
        assert_that!(
            result3,
            matches_pattern!(Entity {
                sparse_index: 2,
                generation: 0,
            })
        );

        assert_that!(
            target,
            matches_pattern!(EntityPool {
                generations: elements_are![eq(&0), eq(&0), eq(&0)],
                free_sparse_indices: is_empty(),
                active_sparse_indices: elements_are![eq(&0), eq(&1), eq(&2)],
                active_sparse_index_locations: elements_are![
                    some(eq(&0)),
                    some(eq(&1)),
                    some(eq(&2))
                ],
            })
        );
    }

    #[test]
    fn should_add_entity_when_a_single_free_sparse_index_exists() {
        // Setup
        let mut target = EntityPool::new();
        let entity = target.add_entity();
        assert_that!(
            entity,
            matches_pattern!(Entity {
                sparse_index: 0,
                generation: 0,
            })
        );
        assert_that!(target.remove_entity(entity), eq(true));

        // Execute
        let result = target.add_entity();

        // Assert
        assert_that!(
            result,
            matches_pattern!(Entity {
                sparse_index: 0,
                generation: 1,
            })
        );

        assert_that!(
            target,
            matches_pattern!(EntityPool {
                generations: elements_are![eq(&1)],
                free_sparse_indices: is_empty(),
                active_sparse_indices: elements_are![eq(&0)],
                active_sparse_index_locations: elements_are![some(eq(&0))],
            })
        );
    }

    #[test]
    fn should_add_single_entity_when_a_multiple_free_sparse_indices_exist() {
        // Setup
        let mut target = EntityPool::new();
        let entity_0 = target.add_entity();
        assert_that!(
            entity_0,
            matches_pattern!(Entity {
                sparse_index: 0,
                generation: 0,
            })
        );
        let entity_1 = target.add_entity();
        assert_that!(
            entity_1,
            matches_pattern!(Entity {
                sparse_index: 1,
                generation: 0,
            })
        );
        let entity_2 = target.add_entity();
        assert_that!(
            entity_2,
            matches_pattern!(Entity {
                sparse_index: 2,
                generation: 0,
            })
        );
        let entity_3 = target.add_entity();
        assert_that!(
            entity_3,
            matches_pattern!(Entity {
                sparse_index: 3,
                generation: 0,
            })
        );
        assert_that!(target.remove_entity(entity_0), eq(true));
        assert_that!(target.remove_entity(entity_3), eq(true));

        // Execute
        let result = target.add_entity();

        // Assert
        assert_that!(
            result,
            matches_pattern!(Entity {
                sparse_index: 3,
                generation: 1,
            })
        );

        assert_that!(
            target,
            matches_pattern!(EntityPool {
                generations: elements_are![eq(&1), eq(&0), eq(&0), eq(&1)],
                free_sparse_indices: elements_are![eq(&0)],
                active_sparse_indices: elements_are![eq(&2), eq(&1), eq(&3)],
                active_sparse_index_locations: elements_are![
                    none(),
                    some(eq(&1)),
                    some(eq(&0)),
                    some(eq(&2))
                ],
            })
        );
    }

    #[test]
    fn should_add_multiple_entities_when_multiple_free_sparse_indices_exist() {
        // Setup
        let mut target = EntityPool::new();
        let entity_0 = target.add_entity();
        assert_that!(
            entity_0,
            matches_pattern!(Entity {
                sparse_index: 0,
                generation: 0,
            })
        );
        let entity_1 = target.add_entity();
        assert_that!(
            entity_1,
            matches_pattern!(Entity {
                sparse_index: 1,
                generation: 0,
            })
        );
        let entity_2 = target.add_entity();
        assert_that!(
            entity_2,
            matches_pattern!(Entity {
                sparse_index: 2,
                generation: 0,
            })
        );
        let entity_3 = target.add_entity();
        assert_that!(
            entity_3,
            matches_pattern!(Entity {
                sparse_index: 3,
                generation: 0,
            })
        );
        assert_that!(target.remove_entity(entity_0), eq(true));
        assert_that!(target.remove_entity(entity_3), eq(true));
        assert_that!(target.remove_entity(entity_1), eq(true));

        // Execute
        let result_1 = target.add_entity();
        let result_2 = target.add_entity();

        // Assert
        assert_that!(
            result_1,
            matches_pattern!(Entity {
                sparse_index: 1,
                generation: 1,
            })
        );
        assert_that!(
            result_2,
            matches_pattern!(Entity {
                sparse_index: 3,
                generation: 1,
            })
        );

        assert_that!(
            target,
            matches_pattern!(EntityPool {
                generations: elements_are![eq(&1), eq(&1), eq(&0), eq(&1)],
                free_sparse_indices: elements_are![eq(&0)],
                active_sparse_indices: elements_are![eq(&2), eq(&1), eq(&3)],
                active_sparse_index_locations: elements_are![
                    none(),
                    some(eq(&1)),
                    some(eq(&0)),
                    some(eq(&2))
                ],
            })
        );
    }

    // Entity removal

    #[test]
    fn should_remove_entity_when_there_is_only_one_entity() {
        // Setup
        let mut target = EntityPool::new();
        let entity_0 = target.add_entity();
        assert_that!(
            entity_0,
            matches_pattern!(Entity {
                sparse_index: 0,
                generation: 0,
            })
        );

        // Execute
        let result = target.remove_entity(entity_0);

        // Assert
        assert_that!(result, eq(true));

        assert_that!(
            target,
            matches_pattern!(EntityPool {
                generations: elements_are![eq(&1)],
                free_sparse_indices: elements_are![eq(&0)],
                active_sparse_indices: is_empty(),
                active_sparse_index_locations: elements_are![none()],
            })
        );
    }

    #[test]
    fn should_remove_entity_and_swap_last_in_when_there_is_multiple_entities() {
        // Setup
        let mut target = EntityPool::new();
        let entity_0 = target.add_entity();
        assert_that!(
            entity_0,
            matches_pattern!(Entity {
                sparse_index: 0,
                generation: 0,
            })
        );
        let entity_1 = target.add_entity();
        assert_that!(
            entity_1,
            matches_pattern!(Entity {
                sparse_index: 1,
                generation: 0,
            })
        );
        let entity_2 = target.add_entity();
        assert_that!(
            entity_2,
            matches_pattern!(Entity {
                sparse_index: 2,
                generation: 0,
            })
        );
        let entity_3 = target.add_entity();
        assert_that!(
            entity_3,
            matches_pattern!(Entity {
                sparse_index: 3,
                generation: 0,
            })
        );

        // Execute
        let result = target.remove_entity(entity_1);

        // Assert
        assert_that!(result, eq(true));

        assert_that!(
            target,
            matches_pattern!(EntityPool {
                generations: elements_are![eq(&0), eq(&1), eq(&0), eq(&0)],
                free_sparse_indices: elements_are![eq(&1)],
                active_sparse_indices: elements_are![eq(&0), eq(&3), eq(&2)],
                active_sparse_index_locations: elements_are![
                    some(eq(&0)),
                    none(),
                    some(eq(&2)),
                    some(eq(&1))
                ],
            })
        );
    }

    #[test]
    fn should_return_false_when_removing_entity_and_it_is_already_removed() {
        // Setup
        let mut target = EntityPool::new();
        let entity = target.add_entity();
        assert_that!(
            entity,
            matches_pattern!(Entity {
                sparse_index: 0,
                generation: 0,
            })
        );
        assert_that!(target.remove_entity(entity), eq(true));

        // Execute
        let result = target.remove_entity(entity);

        // Assert
        assert_that!(result, eq(false));

        assert_that!(
            target,
            matches_pattern!(EntityPool {
                generations: elements_are![eq(&1)],
                free_sparse_indices: elements_are![eq(&0)],
                active_sparse_indices: is_empty(),
                active_sparse_index_locations: elements_are![none()],
            })
        );
    }

    // Entity active check

    #[test]
    fn should_return_false_when_checking_entity_activity_and_entity_is_not_active() {
        // Setup
        let mut target = EntityPool::new();
        let entity = target.add_entity();
        assert_that!(
            entity,
            matches_pattern!(Entity {
                sparse_index: 0,
                generation: 0,
            })
        );
        assert_that!(target.remove_entity(entity), eq(true));

        // Execute
        let result = target.is_entity_active(entity);

        // Assert
        assert_that!(result, eq(false));

        assert_that!(
            target,
            matches_pattern!(EntityPool {
                generations: elements_are![eq(&1)],
                free_sparse_indices: elements_are![eq(&0)],
                active_sparse_indices: is_empty(),
                active_sparse_index_locations: elements_are![none()],
            })
        );
    }

    #[test]
    fn should_return_true_when_checking_entity_activity_and_entity_is_active() {
        // Setup
        let mut target = EntityPool::new();
        let entity = target.add_entity();
        assert_that!(
            entity,
            matches_pattern!(Entity {
                sparse_index: 0,
                generation: 0,
            })
        );

        // Execute
        let result = target.is_entity_active(entity);

        // Assert
        assert_that!(result, eq(true));

        assert_that!(
            target,
            matches_pattern!(EntityPool {
                generations: elements_are![eq(&0)],
                free_sparse_indices: is_empty(),
                active_sparse_indices: elements_are![eq(&0)],
                active_sparse_index_locations: elements_are![some(eq(&0))],
            }),
        );
    }

    // Clear

    #[test]
    fn should_clear_entities() {
        // Setup
        let mut target = EntityPool::new();
        let entity_0 = target.add_entity();
        assert_that!(
            entity_0,
            matches_pattern!(Entity {
                sparse_index: 0,
                generation: 0,
            })
        );
        let entity_1 = target.add_entity();
        assert_that!(
            entity_1,
            matches_pattern!(Entity {
                sparse_index: 1,
                generation: 0,
            })
        );
        let entity_2 = target.add_entity();
        assert_that!(
            entity_2,
            matches_pattern!(Entity {
                sparse_index: 2,
                generation: 0,
            })
        );
        let entity_3 = target.add_entity();
        assert_that!(
            entity_3,
            matches_pattern!(Entity {
                sparse_index: 3,
                generation: 0,
            })
        );

        // Execute
        target.clear();

        // Assert
        assert_that!(
            target,
            matches_pattern!(EntityPool {
                generations: is_empty(),
                free_sparse_indices: is_empty(),
                active_sparse_indices: is_empty(),
                active_sparse_index_locations: is_empty(),
            })
        );
    }

    // Iteration

    #[test]
    fn should_iterate_over_active_entities_when_no_active_entity_exists() {
        // Setup
        let target = EntityPool::new();

        // Execute
        let mut result = target.active_entities();

        // Assert
        assert_that!(result.next(), none());

        assert_that!(
            target,
            matches_pattern!(EntityPool {
                generations: is_empty(),
                free_sparse_indices: is_empty(),
                active_sparse_indices: is_empty(),
                active_sparse_index_locations: is_empty(),
            })
        );
    }

    #[test]
    fn should_iterate_over_active_entities_when_a_single_active_entity_exists() {
        // Setup
        let mut target = EntityPool::new();
        let entity = target.add_entity();
        assert_that!(
            entity,
            matches_pattern!(Entity {
                sparse_index: 0,
                generation: 0,
            })
        );
        assert_that!(target.remove_entity(entity), eq(true));
        let entity_0 = target.add_entity();
        assert_that!(
            entity_0,
            matches_pattern!(Entity {
                sparse_index: 0,
                generation: 1,
            })
        );

        // Execute
        let mut result = target.active_entities();

        // Assert
        assert_that!(result.next(), some(eq(entity_0)));
        assert_that!(result.next(), none());

        assert_that!(
            target,
            matches_pattern!(EntityPool {
                generations: elements_are![eq(&1)],
                free_sparse_indices: is_empty(),
                active_sparse_indices: elements_are![eq(&0)],
                active_sparse_index_locations: elements_are![some(eq(&0))],
            })
        );
    }

    #[test]
    fn should_iterate_over_active_entities_when_many_active_entities_exist() {
        // Setup
        let mut target = EntityPool::new();
        let entity_0 = target.add_entity();
        assert_that!(
            entity_0,
            matches_pattern!(Entity {
                sparse_index: 0,
                generation: 0,
            })
        );
        let entity_1 = target.add_entity();
        assert_that!(
            entity_1,
            matches_pattern!(Entity {
                sparse_index: 1,
                generation: 0,
            })
        );
        let entity_2 = target.add_entity();
        assert_that!(
            entity_2,
            matches_pattern!(Entity {
                sparse_index: 2,
                generation: 0,
            })
        );
        let entity_3 = target.add_entity();
        assert_that!(
            entity_3,
            matches_pattern!(Entity {
                sparse_index: 3,
                generation: 0,
            })
        );
        assert_that!(target.remove_entity(entity_0), eq(true));
        assert_that!(target.remove_entity(entity_3), eq(true));
        assert_that!(target.remove_entity(entity_1), eq(true));
        let entity_1 = target.add_entity();
        assert_that!(
            entity_1,
            matches_pattern!(Entity {
                sparse_index: 1,
                generation: 1,
            })
        );
        let entity_3 = target.add_entity();
        assert_that!(
            entity_3,
            matches_pattern!(Entity {
                sparse_index: 3,
                generation: 1,
            })
        );

        // Execute
        let mut result = target.active_entities();

        // Assert
        assert_that!(result.next(), some(eq(entity_2)));
        assert_that!(result.next(), some(eq(entity_1)));
        assert_that!(result.next(), some(eq(entity_3)));
        assert_that!(result.next(), none());

        assert_that!(
            target,
            matches_pattern!(EntityPool {
                generations: elements_are![eq(&1), eq(&1), eq(&0), eq(&1)],
                free_sparse_indices: elements_are![eq(&0)],
                active_sparse_indices: elements_are![eq(&2), eq(&1), eq(&3)],
                active_sparse_index_locations: elements_are![
                    none(),
                    some(eq(&1)),
                    some(eq(&0)),
                    some(eq(&2))
                ],
            })
        );
    }
}
