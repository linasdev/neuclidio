use neuclidio::engine::NeuclidioEngineBuilder;
use neuclidio::event::NeuclidioEvent;
use winit::window::WindowAttributes;

fn main() {
    env_logger::init();

    let mut neuclidio = NeuclidioEngineBuilder::default()
        .build()
        .expect("Failed to build neuclidio engine");

    let thread = neuclidio.thread(|mut proxy| {
        let window_attributes = WindowAttributes::default();
        proxy
            .add_window_blocking(window_attributes)
            .expect("Failed to add window");

        while let Ok(event) = proxy.poll_for_event() {
            if let Some(NeuclidioEvent::WindowClosed(_)) = event {
                proxy.exit();
            }
        }
    });

    neuclidio.run().expect("Failed to run neuclidio engine");
    thread.join().expect("Failed to join with game thread");
}
