use crate::component::{Component, ComponentExt};
use crate::engine::proxy::EngineProxy;
use crate::engine::proxy::request::EngineProxyRequest;
use crate::engine::render::renderable::Renderable;
use crate::entity::transform::{Transform, TransformExt};
use crate::id::EntityId;
use std::collections::HashSet;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use winit::window::WindowId;

pub mod transform;

#[derive(Clone)]
pub struct Entity {
    id: EntityId,
    window_ids: Arc<Mutex<HashSet<WindowId>>>,
    transform: Arc<Mutex<Transform>>,
    components: Arc<Mutex<Vec<Component>>>,
}

impl Entity {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_transform(transform: Transform) -> Self {
        Self {
            id: EntityId::new(),
            window_ids: Arc::new(Mutex::new(HashSet::new())),
            transform: Arc::new(Mutex::new(transform)),
            components: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn id(&self) -> EntityId {
        self.id
    }

    pub fn has_window_id(&self, window_id: WindowId) -> bool {
        self.window_ids.lock().unwrap().contains(&window_id)
    }

    pub(crate) fn add_window_id(&self, window_id: WindowId) -> bool {
        self.window_ids.lock().unwrap().insert(window_id)
    }

    pub(crate) fn clear_window_ids(&self) {
        self.window_ids.lock().unwrap().clear();
    }

    pub(crate) fn do_with_each_window_id<F>(&self, mut f: F)
    where
        F: FnMut(WindowId),
    {
        for window_id in self.window_ids.lock().unwrap().iter().copied() {
            f(window_id);
        }
    }

    pub fn do_with_transform<T, F>(&self, mut f: F) -> bool
    where
        T: TransformExt,
        F: FnMut(&mut T),
        for<'a> &'a mut Transform: TryInto<&'a mut T>,
    {
        let mut transform = self.transform.lock().unwrap();
        if let Ok(inner_transform) = transform.deref_mut().try_into() {
            f(inner_transform);
            return true;
        }

        false
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

    pub fn add_component<C>(&mut self, component: C, proxy: &EngineProxy)
    where
        C: ComponentExt,
    {
        let component = component.into();
        let renderable = Renderable::try_from(&component);

        self.components.lock().unwrap().push(component);

        if let Ok(renderable) = renderable {
            proxy.send_proxy_request(EngineProxyRequest::HandleRenderableAdded(
                self.id, renderable,
            ));
        }
    }

    pub fn remove_components<C>(&mut self, proxy: &EngineProxy)
    where
        C: ComponentExt,
        for<'a> &'a mut C: TryFrom<&'a Component>,
    {
        let mut components = self.components.lock().unwrap();

        let removed_components =
            components.extract_if(.., |component| <&mut C>::try_from(component).is_err());

        for removed_component in removed_components {
            if let Ok(renderable) = Renderable::try_from(&removed_component) {
                proxy.send_proxy_request(EngineProxyRequest::HandleRenderableRemoved(
                    self.id, renderable,
                ));
            }
        }
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
