use std::collections::HashMap;
use nalgebra::Vector2;
use num_traits::Zero;
use winit::dpi::PhysicalPosition;
use winit::event::{DeviceEvent, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window};

pub type KeyState = ElementState;

pub struct InputManager {
    key_states: HashMap<KeyCode, KeyState>,
    key_just_updated: Vec<KeyCode>,
    button_states: HashMap<MouseButton, ElementState>,
    button_just_updated: Vec<MouseButton>,
    mouse_wheel_delta: f32,
    mouse_pos: PhysicalPosition<f32>,
    mouse_delta: Vector2<f32>, 
    confined: bool,
    lock_on_next_frame: bool,
    unlock_on_next_frame: bool,
    current_mouse_mode: CursorGrabMode,
}

#[allow(unused)]
impl InputManager {
    pub fn new() -> InputManager {
        InputManager {
            key_states: HashMap::default(),
            key_just_updated: Vec::new(),
            button_states: HashMap::default(),
            button_just_updated: Vec::new(),
            mouse_wheel_delta: 0.0,
            mouse_pos: PhysicalPosition::default(),
            mouse_delta: Vector2::zero(),
            confined: false,
            lock_on_next_frame: true,
            unlock_on_next_frame: true,
            current_mouse_mode: CursorGrabMode::None
        }
    }

    pub(crate) fn process_mouse_event(&mut self, window: &Window, device_event: &DeviceEvent) {
        match device_event {
            DeviceEvent::MouseMotion { delta } => {
                self.mouse_delta = Vector2::new(-delta.0 as f32, -delta.1 as f32);
                self.mouse_pos.x += self.mouse_delta.x;
                self.mouse_pos.y += self.mouse_delta.y;
            },
            _ => {}
        }
    }

    pub(crate) fn process_event(&mut self, window: &mut Window, window_event: &WindowEvent) {
        if self.lock_on_next_frame {
            self._set_mouse_mode(window, true);
            self.lock_on_next_frame = false;
        } else if self.unlock_on_next_frame {
            self._set_mouse_mode(window, false);
            self.unlock_on_next_frame = false;
        }
        
        match window_event {
            WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                PhysicalKey::Code(code) => {
                    if !event.state.is_pressed() || self.key_states.get(&code).is_some_and(|state| !state.is_pressed()) {
                        self.key_just_updated.push(code);
                    }
                    
                    self.key_states.insert(code.clone(), event.state);
                }
                _ => {}
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_delta += Vector2::new(self.mouse_pos.x - position.x as f32, self.mouse_pos.y - position.y as f32);
                if self.confined {
                    let size = window.inner_size();
                    let newpos = PhysicalPosition::new(size.width as f64 / 2f64, size.height as f64 / 2f64);
                    if newpos.x == position.x && newpos.y == position.y {
                        return;
                    }
                    self.mouse_pos = PhysicalPosition::new(newpos.x as f32, newpos.y as f32);
                    window.set_cursor_position(newpos);
                } else {
                    self.mouse_pos = PhysicalPosition::new(position.x as f32, position.y as f32);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y as f64,
                    MouseScrollDelta::PixelDelta(pos) => pos.y,
                };
                self.mouse_wheel_delta += y as f32;
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if !state.is_pressed() || self.button_states.get(&button).is_some_and(|state| !state.is_pressed()) {
                    self.button_just_updated.push(button.clone());
                }
                self.button_states.insert(button.clone(), state.clone());
            }
            _ => {}
        }
    }
    
    pub fn get_key_state(&self, key_code: KeyCode) -> KeyState {
        *self.key_states.get(&key_code).unwrap_or(&KeyState::Released)
    }

    // Only is true if the key was JUST pressed
    pub fn is_key_down(&self, key_code: KeyCode) -> bool {
        self.get_key_state(key_code) == KeyState::Pressed && self.key_just_updated.contains(&key_code)
    }

    // true if the key was JUST pressed or is being held
    pub fn is_key_pressed(&self, key_code: KeyCode) -> bool {
        self.get_key_state(key_code) == KeyState::Pressed
    }

    // true if the key was JUST released or is unpressed
    pub fn is_key_released(&self, key_code: KeyCode) -> bool {
        self.get_key_state(key_code) == KeyState::Released && self.key_just_updated.contains(&key_code)
    }

    // Only is true if the key was JUST released
    pub fn is_key_up(&self, key_code: KeyCode) -> bool {
        self.get_key_state(key_code) == KeyState::Released
    }
    
    fn set_mouse_state(&self) {
        //World::instance().
    }
    
    pub fn get_button_state(&self, button: MouseButton) -> ElementState {
        *self.button_states.get(&button).unwrap_or(&ElementState::Released)
    }
    
    pub fn is_button_down(&self, button: MouseButton) -> bool {
        self.get_button_state(button) == ElementState::Pressed && self.button_just_updated.contains(&button)
    }

    pub fn is_button_pressed(&self, button: MouseButton) -> bool {
        self.get_button_state(button) == ElementState::Pressed
    }

    pub fn is_button_released(&self, button: MouseButton) -> bool {
        self.get_button_state(button) == ElementState::Released && self.button_just_updated.contains(&button)
    }
    
    pub fn get_mouse_pos(&self) -> &PhysicalPosition<f32> {
        &self.mouse_pos
    }
    
    pub fn get_mouse_delta(&self) -> &Vector2<f32> {
        &self.mouse_delta
    }
    
    fn _set_mouse_mode(&mut self, window: &mut Window, locked: bool) {
        if locked {
            self.current_mouse_mode = CursorGrabMode::Confined;
            self.confined = true;
            if window.set_cursor_grab(CursorGrabMode::Confined).is_err() {
                window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
                self.current_mouse_mode = CursorGrabMode::Locked;
            }
        } else {
            self.current_mouse_mode = CursorGrabMode::None;
            window.set_cursor_grab(CursorGrabMode::None).unwrap();
            self.confined = false;
        }
        window.set_cursor_visible(!locked);
    }
    
    pub fn set_mouse_mode(&mut self, locked: bool) {
        if locked {
            self.lock_on_next_frame = true;
        } else {
            self.unlock_on_next_frame = true;
        }
    }

    pub fn get_mouse_mode(&self) -> CursorGrabMode {
        self.current_mouse_mode
    }
    
    pub fn next_frame(&mut self) {
        self.key_just_updated.clear();
        self.button_just_updated.clear();
        self.mouse_delta = Vector2::zero();
    }
}
