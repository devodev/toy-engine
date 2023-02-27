use core::object::GameObject;

use cgmath::{Vector3, Vector4};
use engine::engine::{Application, ApplicationContext};

#[derive(Default)]
pub struct EditorApp {}

impl Application for EditorApp {
    fn on_init(&mut self, mut ctx: ApplicationContext) {
        let count = 50;
        for x in 0..count + 1 {
            for y in 0..count + 1 {
                let position = Vector3::new(
                    x as f32 - count as f32 / 2.0,
                    y as f32 - count as f32 / 2.0,
                    1.0,
                );
                let scale = Vector3::new(0.02, 0.015, 1.0);
                let color =
                    Vector4::new(x as f32 / count as f32, y as f32 / count as f32, 0.7, 1.0);

                ctx.add_object(
                    GameObject::new()
                        .with_position(position)
                        .with_scale(scale)
                        .with_color(color),
                );
            }
        }
    }
}
