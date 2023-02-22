pub mod engine;
mod frame_counter;

use std::{error, result};

type Result<T> = result::Result<T, Box<dyn error::Error>>;
