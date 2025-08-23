use crate::assets::{H, HandleName, StoreType};
use kira::sound::static_sound::StaticSoundData;
use std::error::Error;

#[derive(Debug, Clone)]
pub struct Sound {
    pub sound_data: StaticSoundData,
}

impl StoreType for Sound {
    fn name() -> &'static str {
        "Sound"
    }

    fn ident_fmt(handle: H<Self>) -> HandleName<Self> {
        HandleName::Id(handle)
    }

    fn is_builtin(handle: H<Self>) -> bool {
        false
    }
}

impl Sound {
    pub fn load_sound(path: &str) -> Result<Sound, Box<dyn Error>> {
        let data = StaticSoundData::from_file(path);

        let sound = Sound { sound_data: data? };

        Ok(sound)
    }
}
