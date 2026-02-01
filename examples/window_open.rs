use neuclidio::windowing::backend::{WindowingBackendTrait, get_available_windowing_backend};
use neuclidio::windowing::config::WindowConfig;

fn main() {
    let mut windowing_backend =
        get_available_windowing_backend().expect("Failed to get available windowing backend");

    let window_config = WindowConfig::default();
    windowing_backend
        .open(&window_config)
        .expect("Failed to open window");

    loop {
        windowing_backend
            .poll_event()
            .expect("Failed to poll for window events");
    }
}
