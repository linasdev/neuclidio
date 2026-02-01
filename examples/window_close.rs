use neuclidio::windowing::backend::{WindowingBackendTrait, get_available_windowing_backend};
use neuclidio::windowing::config::WindowConfig;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let mut windowing_backend =
        get_available_windowing_backend().expect("Failed to get available windowing backend");

    let window_config = WindowConfig::default();
    let window = windowing_backend
        .open(&window_config)
        .expect("Failed to open window");

    println!("Window opened");
    sleep(Duration::from_secs(5));

    windowing_backend
        .close(window)
        .expect("Failed to close window");

    println!("Window closed");
    sleep(Duration::from_secs(5));

    println!("Exiting");
}
