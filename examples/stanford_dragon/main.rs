use glam::{Quat, Vec3};
use neuclidio::component::mesh::loader::MeshLoader;
use neuclidio::engine::builder::EngineBuilder;
use neuclidio::entity::Entity;
use neuclidio::entity::transform::Transform;
use neuclidio::entity::transform::euclidean::EuclideanTransform;
use neuclidio::event::Event;
use std::time::Instant;
use winit::window::WindowAttributes;

fn main() {
    env_logger::init();

    let mesh = MeshLoader::load_mesh_from_bytes_wavefront(include_bytes!("./mesh.obj"))
        .expect("Failed to load mesh");

    let mut neuclidio = EngineBuilder::new()
        .build()
        .expect("Failed to build Neuclidio engine");

    let thread = neuclidio.thread(move |mut proxy| {
        let window_attributes = WindowAttributes::default();
        let window_id = proxy
            .add_window_blocking(window_attributes)
            .expect("Failed to add window");

        let mut entities = vec![];
        for i in 1..=10 {
            let mut entity = Entity::new_with_transform(Transform::Euclidean(
                EuclideanTransform::default()
                    .with_position(Vec3::new(0.0, 0.0, -5.0 * i as f32))
                    .with_scale(Vec3::ONE * 0.25 * (i as f32).powf(1.3)),
            ));

            entity.add_component(mesh.clone());
            entities.push(entity.clone());
            proxy.add_entity(window_id, entity);
        }

        let start_instant = Instant::now();

        std::thread::spawn(move || {
            loop {
                let amount = (Instant::now() - start_instant).as_secs_f32();
                for (i, entity) in entities.iter().enumerate() {
                    entity.do_with_transform(|transform| {
                        let Transform::Euclidean(transform) = transform;
                        transform.set_rotation(Quat::from_rotation_z(amount + i as f32 * 10.0));
                    });
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });

        while let Ok(event) = proxy.poll_for_event() {
            if let Some(Event::WindowClosed(_)) = event {
                proxy.exit();
            }
        }
    });

    neuclidio.run().expect("Failed to run Neuclidio engine");
    thread.join().expect("Failed to join with game thread");
}
