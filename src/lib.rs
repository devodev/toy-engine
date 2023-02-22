mod camera;
mod component;
mod debug;
mod frame_counter;
mod input;
mod renderer;

// used in sandbox
pub mod engine;
pub mod object;

use std::{error, result};

// used in benches
pub use renderer::QuadBatcher;

type Result<T> = result::Result<T, Box<dyn error::Error>>;
