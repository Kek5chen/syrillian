use crate::World;
use crate::game_thread::GameAppEvent;
use crate::input::gamepad_manager::GamePadManager;
use gilrs::Button;
use log::{info, trace};
use nalgebra::Vector2;
use num_traits::Zero;
use std::collections::HashMap;
use std::sync::mpsc;
use winit::dpi::PhysicalPosition;
use winit::event::{DeviceEvent, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

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
    is_locked: bool,
    auto_cursor_lock: bool,
    quit_on_escape: bool,
    game_event_tx: mpsc::Sender<GameAppEvent>,
}

#[allow(unused)]
impl InputManager {
    pub fn new(game_event_tx: mpsc::Sender<GameAppEvent>) -> Self {
        InputManager {
            key_states: HashMap::default(),
            key_just_updated: Vec::new(),
            button_states: HashMap::default(),
            button_just_updated: Vec::new(),
            gamepad: GamePadManager::default(),
            mouse_wheel_delta: 0.0,
            mouse_pos: PhysicalPosition::default(),
            mouse_delta: Vector2::zero(),
            is_locked: false,
            auto_cursor_lock: false,
            quit_on_escape: false,
            game_event_tx,
        }
    }

    pub(crate) fn process_device_input_event(&mut self, device_event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = device_event {
            self.mouse_delta = Vector2::new(-delta.0 as f32, -delta.1 as f32);
            self.mouse_pos.x += self.mouse_delta.x;
            self.mouse_pos.y += self.mouse_delta.y;
        }
    }

    #[inline]
    pub(crate) fn process_mouse_event(&mut self, position: &PhysicalPosition<f64>) {
        // FIXME: This might not work on windows and/or linux
        let new_pos = PhysicalPosition::new(position.x as f32, position.y as f32);
        self.mouse_pos = new_pos;
    }

    pub fn process_event(&mut self, event: &WindowEvent) {
        self.handle_window_event(event);
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if !event.state.is_pressed()
                        || self
                            .key_states
                            .get(&code)
                            .is_none_or(|state| !state.is_pressed())
                    {
                        self.key_just_updated.push(code);
                    }

                    self.key_states.insert(code, event.state);
                }
            }
            WindowEvent::CursorMoved {
                position,
                device_id,
            } => self.process_mouse_event(position),
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
                        .is_none_or(|state| !state.is_pressed())
                {
                    self.button_just_updated.push(*button);
                }
                self.button_states.insert(*button, *state);
            }
            _ => {}
        }
    }

    pub fn key_state(&self, key_code: KeyCode) -> KeyState {
        *self
            .key_states
            .get(&key_code)
            .unwrap_or(&KeyState::Released)
    }

    // Only is true if the key was JUST pressed
    pub fn is_key_down(&self, key_code: KeyCode) -> bool {
        self.key_state(key_code) == KeyState::Pressed && self.key_just_updated.contains(&key_code)
    }

    // true if the key was JUST pressed or is being held
    pub fn is_key_pressed(&self, key_code: KeyCode) -> bool {
        self.key_state(key_code) == KeyState::Pressed
    }

    // true if the key was JUST released or is unpressed
    pub fn is_key_released(&self, key_code: KeyCode) -> bool {
        self.key_state(key_code) == KeyState::Released && self.key_just_updated.contains(&key_code)
    }

    // Only is true if the key was JUST released
    pub fn is_key_up(&self, key_code: KeyCode) -> bool {
        self.key_state(key_code) == KeyState::Released
    }

    pub fn button_state(&self, button: MouseButton) -> ElementState {
        *self
            .button_states
            .get(&button)
            .unwrap_or(&ElementState::Released)
    }

    pub fn is_button_down(&self, button: MouseButton) -> bool {
        self.button_state(button) == ElementState::Pressed
            && self.button_just_updated.contains(&button)
    }

    pub fn is_button_pressed(&self, button: MouseButton) -> bool {
        self.button_state(button) == ElementState::Pressed
    }

    pub fn is_button_released(&self, button: MouseButton) -> bool {
        self.button_state(button) == ElementState::Released
            && self.button_just_updated.contains(&button)
    }

    #[inline]
    pub fn mouse_position(&self) -> PhysicalPosition<f32> {
        self.mouse_pos
    }

    pub fn mouse_delta(&self) -> &Vector2<f32> {
        &self.mouse_delta
    }

    pub fn lock_cursor(&mut self) {
        trace!("GT: Locked cursor");
        self.is_locked = true;
        self.game_event_tx
            .send(GameAppEvent::cursor_mode(true, false));
    }

    pub fn unlock_cursor(&mut self) {
        trace!("GT: Unlocked cursor");
        self.is_locked = false;
        self.game_event_tx
            .send(GameAppEvent::cursor_mode(false, true));
    }

    pub fn is_cursor_locked(&self) -> bool {
        self.is_locked
    }

    pub fn next_frame(&mut self) {
        if self.quit_on_escape && self.is_key_down(KeyCode::Escape) && !self.is_cursor_locked() {
            info!("Shutting down world from escape press");
            World::instance().shutdown();
        }
        if self.auto_cursor_lock {
            self.auto_cursor_lock_loop();
        }

        self.key_just_updated.clear();
        self.button_just_updated.clear();
        self.mouse_delta = Vector2::zero();
        self.gamepad.poll();
    }

    pub fn set_auto_cursor_lock(&mut self, enabled: bool) {
        self.auto_cursor_lock = enabled
    }

    pub fn set_quit_on_escape(&mut self, enabled: bool) {
        self.quit_on_escape = enabled;
    }

    fn auto_cursor_lock_loop(&mut self) {
        if self.is_key_down(KeyCode::Escape) {
            self.unlock_cursor();
        }

        if self.is_button_down(MouseButton::Left) || self.is_button_down(MouseButton::Right) {
            self.lock_cursor();
        }
    }

    pub fn is_sprinting(&self) -> bool {
        self.is_key_pressed(KeyCode::ShiftLeft) || self.gamepad.is_button_pressed(Button::LeftThumb)
    }

    pub fn is_jump_down(&self) -> bool {
        self.is_key_down(KeyCode::Space) || self.gamepad.is_button_down(Button::South)
    }
}
