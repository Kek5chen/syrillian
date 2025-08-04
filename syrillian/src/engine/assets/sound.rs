use std::error::Error;
use std::path::Path;
use kira::sound::static_sound::StaticSoundData;
use crate::assets::{HandleName, Store, StoreDefaults, StoreType, H};
use crate::audio::audio::AudioAsset;

#[derive(Debug, Clone)]
pub struct Sound {
    sound_data: StaticSoundData,
}

impl H<Sound> {

}

impl StoreDefaults for Sound {
    fn populate(store: &mut Store<Self>) {

    }
}

impl StoreType for Sound {
    fn name() -> &'static str {
        "Sound"
    }

    fn ident_fmt(handle: H<Self>) -> HandleName<Self> {
        todo!()
    }
}

impl Sound {
    fn load_sound(path: &str) -> Result<Sound, Box<dyn Error>> {
        let data = StaticSoundData::from_file(path);

        let sound = Sound {
            sound_data: data?
        };

        Ok(sound)
    }

}
