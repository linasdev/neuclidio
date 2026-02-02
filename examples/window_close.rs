use neuclidio::engine::NeuclidioEngineBuilder;
use std::thread::sleep;
use std::time::Duration;
use winit::window::WindowAttributes;

fn main() {
    env_logger::init();

    let mut neuclidio = NeuclidioEngineBuilder::default()
        .build()
        .expect("Failed to build neuclidio engine");

    let thread = neuclidio.thread(|proxy| {
        let window_attributes = WindowAttributes::default();
        let window_id = proxy
            .add_window_blocking(window_attributes)
            .expect("Failed to add window");

        println!("Window opened");
        sleep(Duration::from_secs(5));

        proxy
            .close_window_blocking(window_id)
            .expect("Failed to close window");

        println!("Window closed");
        sleep(Duration::from_secs(5));

        println!("Exiting");
        proxy.exit();
    });

    neuclidio.run().expect("Failed to run neuclidio engine");
    thread.join().expect("Failed to join with game thread");
}
