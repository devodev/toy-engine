use core::object::GameObject;

use cgmath::{Vector3, Vector4};
use engine::engine::{Application, ApplicationContext, EngineBuilder};
use log::LevelFilter;
use winit::dpi::LogicalSize;
use winit::window::WindowBuilder;

const WINDOW_TITLE: &str = "Sandbox";
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

fn main() {
    // initialize logger
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();

    // setup window
    let window_builder = {
        let logical_window_size: LogicalSize<u32> = (WINDOW_WIDTH, WINDOW_HEIGHT).into();
        WindowBuilder::new()
            .with_title(WINDOW_TITLE)
            .with_inner_size(logical_window_size)
            .with_resizable(true)
    };

    // setup sandbox impl
    let application = Sandbox::default();

    // setup engine
    let mut engine = EngineBuilder::new(Box::new(application))
        .with_window_builder(Some(window_builder))
        .build()
        .expect("engine builder builds");

    // start engine
    engine.run()
}

#[derive(Default)]
struct Sandbox {}

impl Application for Sandbox {
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
