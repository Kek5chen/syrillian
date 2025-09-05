use gilrs::{Axis, Button, Event, EventType, Gilrs, GilrsBuilder};
use log::{debug, trace};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug)]
pub struct GamePadManager {
    poller: Gilrs,
    axis: HashMap<Axis, f32>,
    buttons: HashMap<Button, f32>,
    buttons_just_updated: Vec<Button>,
}

impl Default for GamePadManager {
    fn default() -> Self {
        //let poller = Gilrs::new().expect("Init gamepad input failed");
        let poller = match GilrsBuilder::new()
            .add_env_mappings(true)
            .add_included_mappings(false)
            .build()
        {
            Ok(g) => g,
            Err(err) => {
                log::error!("An error was in creating a Gilrs : {}", err);
                std::process::exit(1);
            }
        };

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
            trace!("[Gamepads] Handling Gamepad Event: {event:?}");
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
        let uuid = Uuid::from_bytes(gamepad.uuid());
        match event.event {
            EventType::Connected => debug!("[Gamepads] Connected Gamepad: {name} ({uuid})"),
            EventType::Disconnected => debug!("[Gamepads] Disconnected Gamepad {name} ({uuid})"),
            _ => (),
        }
    }

    pub fn handle_gamepad_event(&mut self, event: &EventType) {
        match event {
            EventType::ButtonPressed(button, ..) | EventType::ButtonRepeated(button, ..) => {
                self.buttons.insert(*button, 1.0);
                self.buttons_just_updated.push(*button);
            }
            EventType::ButtonReleased(button, ..) => {
                self.buttons.insert(*button, 0.0);
                self.buttons_just_updated.push(*button);
            }
            EventType::ButtonChanged(button, value, ..) => {
                self.buttons.insert(*button, *value);
                self.buttons_just_updated.push(*button);
            }
            EventType::AxisChanged(axis, value, ..) => {
                self.axis.insert(*axis, *value);
            }
            _ => {}
        }
    }

    pub fn axis(&self, axis: Axis) -> f32 {
        self.axis.get(&axis).copied().unwrap_or(0.0)
    }

    pub fn button(&self, button: Button) -> f32 {
        self.buttons.get(&button).copied().unwrap_or(0.0)
    }

    pub fn is_button_pressed(&self, button: Button) -> bool {
        self.button(button) > 0.5
    }

    pub fn is_button_down(&self, button: Button) -> bool {
        self.is_button_pressed(button) && self.buttons_just_updated.contains(&button)
    }

    pub fn is_button_released(&self, button: Button) -> bool {
        !self.is_button_pressed(button) && self.buttons_just_updated.contains(&button)
    }

    pub fn next_frame(&mut self) {
        self.buttons_just_updated.clear();
    }
}
