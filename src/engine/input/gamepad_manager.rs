use gilrs::{Axis, Button, Event, EventType, Gilrs};
use log::debug;
use std::collections::HashMap;

#[derive(Debug)]
pub struct GamePadManager {
    poller: Gilrs,
    axis: HashMap<Axis, f32>,
    buttons: HashMap<Button, bool>,
    buttons_just_updated: Vec<Button>,
}

impl Default for GamePadManager {
    fn default() -> Self {
        let poller = Gilrs::new().expect("Init gamepad input failed");

        Self {
            poller,
            axis: HashMap::new(),
            buttons: HashMap::new(),
            buttons_just_updated: Vec::new(),
        }
    }
}

impl GamePadManager {
    pub fn poll(&mut self) {
        self.next_frame();
        while let Some(event) = self.poller.next_event() {
            match event.event {
                EventType::Connected | EventType::Disconnected => {
                    self.handle_device_meta_event(&event)
                }
                _ => self.handle_gamepad_event(&event.event),
            }
        }
    }

    fn handle_device_meta_event(&self, event: &Event) {
        let gamepad = self.poller.gamepad(event.id);
        let name = gamepad.name();
        let uuid = gamepad.uuid().map(|num| format!("{num:03}")).join("");
        match event.event {
            EventType::Connected => debug!("[Gamepads] Connected Gamepad: {name} ({uuid})"),
            EventType::Disconnected => {
                debug!("[Gamepads] Disconnected Gamepad {name} ({uuid})");
                return;
            }
            _ => (),
        }
    }

    pub fn handle_gamepad_event(&mut self, event: &EventType) {
        match event {
            EventType::ButtonPressed(button, code) | EventType::ButtonRepeated(button, code) => {
                debug!("[Gamepads] ButtonPressed {button:?} ({code:?})");
                self.buttons.insert(*button, true);
                self.buttons_just_updated.push(*button);
            }
            EventType::ButtonReleased(button, code) => {
                debug!("[Gamepads] ButtonReleased {button:?} ({code:?})");
                self.buttons.insert(*button, false);
                self.buttons_just_updated.push(*button);
            }
            EventType::ButtonChanged(button, value, code) => {
                debug!("[Gamepads] ButtonChanged {button:?} ({code:?})");
                self.buttons.insert(*button, *value >= 0.5);
                self.buttons_just_updated.push(*button);
            }
            EventType::AxisChanged(axis, value, code) => {
                debug!("{code:?}");
                self.axis.insert(*axis, *value);
            }
            _ => {}
        }
    }

    pub fn axis(&self, axis: Axis) -> f32 {
        self.axis.get(&axis).copied().unwrap_or(0.0)
    }

    pub fn button(&self, button: Button) -> bool {
        self.buttons.get(&button).copied().unwrap_or(false)
    }

    pub fn button_down(&self, button: Button) -> bool {
        self.buttons.get(&button).copied().unwrap_or(false) && self.buttons_just_updated.contains(&button)
    }

    pub fn button_up(&self, button: Button) -> bool {
        !self.buttons.get(&button).copied().unwrap_or(false) && self.buttons_just_updated.contains(&button)
    }

    pub fn next_frame(&mut self) {
        self.buttons_just_updated.clear();
    }
}
