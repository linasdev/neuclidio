use crate::component::{Component, ComponentExt};
use crate::engine::render::renderable::Renderable;
use crate::entity::transform::Transform;
use crate::id_generator::IdGenerator;
use std::sync::{Arc, Mutex};

pub mod transform;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub struct EntityId(pub(crate) u64);

#[derive(Clone)]
pub struct Entity {
    id: EntityId,
    transform: Arc<Mutex<Transform>>,
    components: Arc<Mutex<Vec<Component>>>,
}

impl Entity {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_transform(transform: Transform) -> Self {
        Self {
            id: IdGenerator::generate_entity_id(),
            transform: Arc::new(Mutex::new(transform)),
            components: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn id(&self) -> EntityId {
        self.id
    }

    pub fn do_with_transform<F>(&self, mut f: F)
    where
        F: FnMut(&mut Transform),
    {
        let mut transform = self.transform.lock().unwrap();
        f(&mut transform);
    }

    pub fn do_with_components<C, F>(&self, mut f: F) -> bool
    where
        C: ComponentExt,
        F: FnMut(&mut C),
        for<'a> &'a mut Component: TryInto<&'a mut C>,
    {
        let mut invoked_at_least_once = false;

        for component in self.components.lock().unwrap().iter_mut() {
            if let Ok(inner_component) = component.try_into() {
                f(inner_component);
                invoked_at_least_once = true;
            }
        }

        invoked_at_least_once
    }

    pub fn add_component<C>(&mut self, component: C)
    where
        C: Into<Component>,
    {
        self.components.lock().unwrap().push(component.into());
    }

    pub(crate) fn get_renderables(&self) -> Vec<Renderable> {
        let mut renderables = vec![];

        for component in self.components.lock().unwrap().iter() {
            if let Ok(renderable) = component.try_into() {
                renderables.push(renderable);
            }
        }

        renderables
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self::new_with_transform(Transform::default())
    }
}
