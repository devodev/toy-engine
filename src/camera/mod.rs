mod controller;
mod ortho;
mod perspective;

use cgmath::Matrix4;

pub use self::controller::CameraController;
pub use self::ortho::CameraOrthographic;
pub use self::perspective::CameraPerspective;

pub trait Camera {
    fn projection_matrix(&self) -> Matrix4<f32>;
    fn set_zoom(&mut self, amount: f32);
    fn zoom(&self) -> f32;
    fn reset_zoom(&mut self);
    fn resize(&mut self, width: u32, height: u32);
}
