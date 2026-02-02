use crate::engine::proxy::NeuclidioEngineProxy;
use std::thread::JoinHandle;

pub struct NeuclidioEngineThread<T> {
    join_handle: JoinHandle<T>,
}

impl<T: Send + 'static> NeuclidioEngineThread<T> {
    pub(crate) fn new<F>(proxy: NeuclidioEngineProxy, f: F) -> Self
    where
        F: FnOnce(NeuclidioEngineProxy) -> T,
        F: Send + 'static,
    {
        let join_handle = std::thread::spawn(|| f(proxy));

        Self { join_handle }
    }

    pub fn join(self) -> std::thread::Result<T> {
        self.join_handle.join()
    }
}
