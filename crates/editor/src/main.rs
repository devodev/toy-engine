use editor::EditorApp;
use engine::engine::EngineBuilder;
use log::LevelFilter;
use winit::dpi::LogicalSize;
use winit::window::WindowBuilder;

const WINDOW_TITLE: &str = "Editor";
const WINDOW_WIDTH: u32 = 1024;
const WINDOW_HEIGHT: u32 = 768;

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

    // setup application
    let application = EditorApp::default();

    // setup engine
    let mut engine = EngineBuilder::new(Box::new(application))
        .with_window_builder(Some(window_builder))
        .build()
        .expect("engine builder builds");

    // start engine
    engine.run()
}
