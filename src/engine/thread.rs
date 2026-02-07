use crate::engine::proxy::EngineProxy;
use std::thread::JoinHandle;

pub struct EngineThread<T> {
    join_handle: JoinHandle<T>,
}

impl<T: Send + 'static> EngineThread<T> {
    pub(crate) fn new<F>(proxy: EngineProxy, f: F) -> Self
    where
        F: FnOnce(EngineProxy) -> T,
        F: Send + 'static,
    {
        let join_handle = std::thread::spawn(|| f(proxy));

        Self { join_handle }
    }

    pub fn join(self) -> std::thread::Result<T> {
        self.join_handle.join()
    }
}
