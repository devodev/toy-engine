use std::ops::{Add, Mul, Sub};
use std::time;

use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Point3, SquareMatrix, Vector3};
use winit::event::{Event, VirtualKeyCode, WindowEvent};

use super::Camera;
use crate::input::InputSystem;

const HORIZONTAL_VEC: Vector3<f32> = Vector3::new(1.0, 0.0, 0.0);
const VERTICAL_VEC: Vector3<f32> = Vector3::new(0.0, 1.0, 0.0);

#[derive(Debug, Clone)]
pub struct CameraController<T: Camera> {
    camera: T,

    speed_base: f32,
    pos: Vector3<f32>,
    target: Vector3<f32>,
    up: Vector3<f32>,

    view: Matrix4<f32>,

    zoom_target: f32,
    zoom_min: f32,
    zoom_max: f32,
    zoom_sensitivity: f32,
    zoom_speed: f32,
}

#[allow(unused)]
impl<T> CameraController<T>
where
    T: Camera,
{
    pub fn new(camera: T) -> Self {
        let initial_zoom = camera.zoom();
        let mut controller = Self {
            camera,
            speed_base: 1.0,
            pos: Vector3::new(0.0, 0.0, 10.0),
            target: Vector3::new(0.0, 0.0, -1.0),
            up: Vector3::new(0.0, 1.0, 0.0),
            view: Matrix4::identity(),
            zoom_target: initial_zoom,
            zoom_min: 0.01,
            zoom_max: 10.0,
            zoom_sensitivity: 0.1,
            zoom_speed: 10.0,
        };
        controller.compute_view_matrix();
        controller
    }

    pub fn on_update(&mut self, input: &InputSystem, delta: time::Duration) {
        self.compute_view_matrix();

        let speed = self.speed_base * delta.as_secs_f32();

        // adapt movement speed based on zoom level
        let movement_speed = {
            let current_zoom = self.camera.zoom();
            let zoom_magnitute = self.zoom_max - self.zoom_min;
            let zoom_level = current_zoom / zoom_magnitute;

            let speed_buf_min = 0.01;
            let speed_buf_max = 0.8;
            let speed_modifier_scale = 50.0;
            let speed_modifier =
                speed_buf_min + zoom_level * (speed_buf_max - speed_buf_min) * speed_modifier_scale;

            speed * speed_modifier
        };

        // move camera => Z
        if input.is_key_pressed(VirtualKeyCode::Q) {
            self.move_backward(movement_speed)
        }
        if input.is_key_pressed(VirtualKeyCode::E) {
            self.move_forward(movement_speed)
        }
        // move camera => Y
        if input.is_key_pressed(VirtualKeyCode::W) {
            self.move_up(movement_speed)
        }
        if input.is_key_pressed(VirtualKeyCode::S) {
            self.move_down(movement_speed)
        }
        // move camera => X
        if input.is_key_pressed(VirtualKeyCode::A) {
            self.move_left(movement_speed)
        }
        if input.is_key_pressed(VirtualKeyCode::D) {
            self.move_right(movement_speed)
        }

        // on scroll, update zoom_target
        self.zoom_target -= input.mouse_scoll_y() * self.zoom_sensitivity;

        // clamp zoom target between min and max
        self.zoom_target = clamp(self.zoom_target, self.zoom_min, self.zoom_max);

        // move camera (lerp) towards zoom_target
        let zoom_amount = lerp(
            self.camera.zoom(),
            self.zoom_target,
            speed * self.zoom_speed,
        );
        self.camera.set_zoom(zoom_amount);

        // reset zoom
        if input.is_key_pressed(VirtualKeyCode::Z) {
            self.camera.reset_zoom();
            self.zoom_target = self.camera.zoom();
        }
    }

    pub fn on_event(&mut self, event: &Event<()>) {
        #[allow(clippy::single_match)]
        #[allow(clippy::collapsible_match)]
        match event {
            Event::WindowEvent { ref event, .. } => match *event {
                // handle window resized
                WindowEvent::Resized(size) => self.camera.resize(size.width, size.height),
                _ => {}
            },
            _ => {}
        }
    }

    pub fn view_projection_matrix(&self) -> Matrix4<f32> {
        self.camera.projection_matrix().mul(self.view)
    }

    fn compute_view_matrix(&mut self) {
        self.view = Matrix4::look_at_rh(
            Point3::from_vec(self.pos),
            Point3::from_vec(self.pos.add(self.target)),
            self.up,
        )
    }

    fn move_forward(&mut self, speed: f32) {
        self.pos = self.pos.add(self.target.mul(speed))
    }
    fn move_backward(&mut self, speed: f32) {
        self.pos = self.pos.sub(self.target.mul(speed))
    }

    fn move_left(&mut self, speed: f32) {
        self.pos = self
            .pos
            .sub(self.target.normalize().cross(VERTICAL_VEC).mul(speed))
    }
    fn move_right(&mut self, speed: f32) {
        self.pos = self
            .pos
            .add(self.target.normalize().cross(VERTICAL_VEC).mul(speed))
    }
    fn move_up(&mut self, speed: f32) {
        self.pos = self
            .pos
            .add(self.target.normalize().cross(HORIZONTAL_VEC).mul(speed))
    }
    fn move_down(&mut self, speed: f32) {
        self.pos = self
            .pos
            .sub(self.target.normalize().cross(HORIZONTAL_VEC).mul(speed))
    }
}

fn clamp(v: f32, min: f32, max: f32) -> f32 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    start + (end - start) * amount
}
