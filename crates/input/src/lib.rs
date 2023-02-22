use std::collections::HashMap;

use winit::event::{
    DeviceEvent, ElementState, Event, KeyboardInput, MouseScrollDelta, VirtualKeyCode, WindowEvent,
};

#[derive(Default, Debug)]
struct ScrollState {
    x: f32,
    y: f32,
}

#[derive(Default, Debug)]
pub struct InputSystem {
    focused: bool,

    keyboard: HashMap<VirtualKeyCode, ElementState>,
    scroll_state: ScrollState,
}

impl InputSystem {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn reset(&mut self) {
        self.scroll_state = ScrollState::default();
    }

    pub fn on_event(&mut self, event: &Event<()>) {
        // handle focus state early
        if let Event::WindowEvent {
            event: WindowEvent::Focused(f),
            ..
        } = event
        {
            self.focused = *f;
            // when losing focus, reset states
            if !f {
                self.keyboard.clear();
                self.scroll_state = ScrollState::default();
            }
            return;
        }

        // bail out if we are not focused
        if !self.focused {
            return;
        }

        #[allow(clippy::single_match)]
        #[allow(clippy::collapsible_match)]
        match event {
            Event::MainEventsCleared => self.reset(),
            Event::WindowEvent { ref event, .. } => match *event {
                // handle keys
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state,
                            virtual_keycode,
                            ..
                        },
                    ..
                } => {
                    if let Some(keycode) = virtual_keycode {
                        self.keyboard.insert(keycode, state);
                    }
                }
                _ => {}
            },
            Event::DeviceEvent { ref event, .. } => match *event {
                // handle mouse scroll
                DeviceEvent::MouseWheel {
                    delta: MouseScrollDelta::LineDelta(delta_x, delta_y),
                } => {
                    if delta_x != 0.0 {
                        self.scroll_state.x = delta_x.signum();
                    }
                    if delta_y != 0.0 {
                        self.scroll_state.y = delta_y.signum();
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub fn is_key_pressed(&self, key: VirtualKeyCode) -> bool {
        match self.keyboard.get(&key) {
            Some(state) => state == &ElementState::Pressed,
            None => false,
        }
    }

    #[allow(unused)]
    pub fn is_key_released(&self, key: VirtualKeyCode) -> bool {
        match self.keyboard.get(&key) {
            Some(state) => state == &ElementState::Released,
            None => true,
        }
    }

    #[allow(unused)]
    pub fn mouse_scoll_x(&self) -> f32 {
        self.scroll_state.x
    }

    pub fn mouse_scoll_y(&self) -> f32 {
        self.scroll_state.y
    }
}
