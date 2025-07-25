use crate::input::gamepad_manager::GamePadManager;
use crate::World;
use gilrs::Button;
use nalgebra::Vector2;
use num_traits::Zero;
use std::collections::HashMap;
use winit::dpi::PhysicalPosition;
use winit::event::{DeviceEvent, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window};

pub type KeyState = ElementState;

#[derive(Debug)]
pub struct InputManager {
    key_states: HashMap<KeyCode, KeyState>,
    key_just_updated: Vec<KeyCode>,
    button_states: HashMap<MouseButton, ElementState>,
    button_just_updated: Vec<MouseButton>,
    pub gamepad: GamePadManager,
    mouse_wheel_delta: f32,
    mouse_pos: PhysicalPosition<f32>,
    mouse_delta: Vector2<f32>,
    lock_on_next_frame: bool,
    unlock_on_next_frame: bool,
    current_mouse_mode: CursorGrabMode,
    auto_cursor_lock: bool,
    quit_on_escape: bool,
}

impl Default for InputManager {
    fn default() -> Self {
        InputManager {
            key_states: HashMap::default(),
            key_just_updated: Vec::new(),
            button_states: HashMap::default(),
            button_just_updated: Vec::new(),
            gamepad: GamePadManager::default(),
            mouse_wheel_delta: 0.0,
            mouse_pos: PhysicalPosition::default(),
            mouse_delta: Vector2::zero(),
            lock_on_next_frame: true,
            unlock_on_next_frame: true,
            current_mouse_mode: CursorGrabMode::None,
            auto_cursor_lock: false,
            quit_on_escape: false,
        }
    }
}

#[allow(unused)]
impl InputManager {
    pub(crate) fn process_device_input_event(
        &mut self,
        window: &Window,
        device_event: &DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = device_event {
            self.mouse_delta = Vector2::new(-delta.0 as f32, -delta.1 as f32);
            self.mouse_pos.x += self.mouse_delta.x;
            self.mouse_pos.y += self.mouse_delta.y;
        }
    }

    #[inline]
    pub(crate) fn process_mouse_event(
        &mut self,
        window: &Window,
        position: &PhysicalPosition<f64>,
    ) {
        #[cfg(not(target_os = "macos"))]
        {
            self.mouse_delta += Vector2::new(
                self.mouse_pos.x - position.x as f32,
                self.mouse_pos.y - position.y as f32,
            );
            if self.is_cursor_locked() {
                let size = window.inner_size();
                let newpos =
                    PhysicalPosition::new(size.width as f64 / 2f64, size.height as f64 / 2f64);
                if newpos.x == position.x && newpos.y == position.y {
                    return;
                }
                self.mouse_pos = PhysicalPosition::new(newpos.x as f32, newpos.y as f32);
                window.set_cursor_position(newpos);
            } else {
                self.mouse_pos = PhysicalPosition::new(position.x as f32, position.y as f32);
            }
        }

        #[cfg(target_os = "macos")]
        {
            self.mouse_pos = PhysicalPosition::new(position.x as f32, position.y as f32);
        }
    }

    pub fn process_event(&mut self, window: &mut Window, event: &WindowEvent) {
        if self.auto_cursor_lock {
            self.auto_cursor_lock_loop();
        }
        if self.quit_on_escape && self.is_key_down(KeyCode::Escape) && !self.is_cursor_locked() {
            World::instance().shutdown();
        }
        self.do_cursor_lock(window);

        self.handle_window_event(window, event);
        self.gamepad.poll();
    }

    pub fn handle_window_event(&mut self, window: &mut Window, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if !event.state.is_pressed()
                        || self
                            .key_states
                            .get(&code)
                            .is_some_and(|state| !state.is_pressed())
                    {
                        self.key_just_updated.push(code);
                    }

                    self.key_states.insert(code, event.state);
                }
            }
            WindowEvent::CursorMoved { position, .. } => self.process_mouse_event(window, position),
            WindowEvent::MouseWheel { delta, .. } => {
                let y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y as f64,
                    MouseScrollDelta::PixelDelta(pos) => pos.y,
                };
                self.mouse_wheel_delta += y as f32;
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if !state.is_pressed()
                    || self
                        .button_states
                        .get(button)
                        .is_some_and(|state| !state.is_pressed())
                {
                    self.button_just_updated.push(*button);
                }
                self.button_states.insert(*button, *state);
            }
            _ => {}
        }
    }

    pub fn get_key_state(&self, key_code: KeyCode) -> KeyState {
        *self
            .key_states
            .get(&key_code)
            .unwrap_or(&KeyState::Released)
    }

    // Only is true if the key was JUST pressed
    pub fn is_key_down(&self, key_code: KeyCode) -> bool {
        self.get_key_state(key_code) == KeyState::Pressed
            && self.key_just_updated.contains(&key_code)
    }

    // true if the key was JUST pressed or is being held
    pub fn is_key_pressed(&self, key_code: KeyCode) -> bool {
        self.get_key_state(key_code) == KeyState::Pressed
    }

    // true if the key was JUST released or is unpressed
    pub fn is_key_released(&self, key_code: KeyCode) -> bool {
        self.get_key_state(key_code) == KeyState::Released
            && self.key_just_updated.contains(&key_code)
    }

    // Only is true if the key was JUST released
    pub fn is_key_up(&self, key_code: KeyCode) -> bool {
        self.get_key_state(key_code) == KeyState::Released
    }

    fn set_mouse_state(&self) {
        //World::instance().
    }

    pub fn get_button_state(&self, button: MouseButton) -> ElementState {
        *self
            .button_states
            .get(&button)
            .unwrap_or(&ElementState::Released)
    }

    pub fn is_button_down(&self, button: MouseButton) -> bool {
        self.get_button_state(button) == ElementState::Pressed
            && self.button_just_updated.contains(&button)
    }

    pub fn is_button_pressed(&self, button: MouseButton) -> bool {
        self.get_button_state(button) == ElementState::Pressed
    }

    pub fn is_button_released(&self, button: MouseButton) -> bool {
        self.get_button_state(button) == ElementState::Released
            && self.button_just_updated.contains(&button)
    }

    #[inline]
    pub fn mouse_position(&self) -> PhysicalPosition<f32> {
        self.mouse_pos
    }

    pub fn mouse_delta(&self) -> &Vector2<f32> {
        &self.mouse_delta
    }

    fn _set_cursor_grab(
        &mut self,
        window: &mut Window,
        mode: CursorGrabMode,
    ) -> Result<(), winit::error::ExternalError> {
        window.set_cursor_grab(mode)?;
        window.set_cursor_visible(mode == CursorGrabMode::None);
        self.current_mouse_mode = mode;
        Ok(())
    }

    fn _lock_cursor(&mut self, window: &mut Window) {
        self._set_cursor_grab(window, CursorGrabMode::Locked)
            .or_else(|_| self._set_cursor_grab(window, CursorGrabMode::Confined))
            .expect("Couldn't lock or confine the cursor");
    }

    fn _unlock_cursor(&mut self, window: &mut Window) {
        self._set_cursor_grab(window, CursorGrabMode::None)
            .expect("Couldn't grab the cursor");
    }

    pub fn lock_cursor(&mut self, locked: bool) {
        if locked {
            self.lock_on_next_frame = true;
        } else {
            self.unlock_on_next_frame = true;
        }
    }

    pub fn is_cursor_locked(&self) -> bool {
        self.current_mouse_mode != CursorGrabMode::None
    }

    pub fn next_frame(&mut self) {
        self.key_just_updated.clear();
        self.button_just_updated.clear();
        self.mouse_delta = Vector2::zero();
    }

    pub fn set_auto_cursor_lock(&mut self, enabled: bool) {
        self.auto_cursor_lock = enabled
    }

    pub fn set_quit_on_escape(&mut self, enabled: bool) {
        self.quit_on_escape = enabled;
    }

    fn auto_cursor_lock_loop(&mut self) {
        if self.is_key_down(KeyCode::Escape) {
            self.lock_cursor(false);
        }

        if self.is_button_pressed(MouseButton::Left) || self.is_button_pressed(MouseButton::Right) {
            self.lock_cursor(true);
        }
    }

    fn do_cursor_lock(&mut self, window: &mut Window) {
        if self.lock_on_next_frame {
            self._lock_cursor(window);
            self.lock_on_next_frame = false;
        } else if self.unlock_on_next_frame {
            self._unlock_cursor(window);
            self.unlock_on_next_frame = false;
        }
    }

    pub fn is_sprinting(&self) -> bool {
        if cfg!(target_os = "macos") {
            if self.gamepad.is_button_pressed(Button::Unknown) {
                return true;
            }
        }

        self.is_key_pressed(KeyCode::ShiftLeft) || self.gamepad.is_button_pressed(Button::LeftThumb)
    }

    pub fn is_jump_down(&self) -> bool {
        self.is_key_down(KeyCode::Space) || self.gamepad.is_button_down(Button::South)
    }
}
