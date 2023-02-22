use cgmath::{Vector3, Vector4};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Vector3<f32>,
    pub scale: Vector3<f32>,
}

impl Transform {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub color: Vector4<f32>,
}

impl Color {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Color {
    fn default() -> Self {
        Self {
            color: Vector4::new(0.0, 0.0, 0.0, 0.0),
        }
    }
}
