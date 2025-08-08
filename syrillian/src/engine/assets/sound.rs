use std::error::Error;
use std::rc::Rc;
use kira::sound::static_sound::StaticSoundData;
use kira::track::SpatialTrackHandle;
use nalgebra::Vector3;
use crate::assets::{HandleName, Store, StoreDefaults, StoreType, H};

#[derive(Debug, Clone)]
pub struct Sound {
    pub sound_data: StaticSoundData,
    pub loops: bool,
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
        HandleName::Id(handle)
    }

    fn is_builtin(handle: H<Self>) -> bool {
        false
    }
}

impl Sound {
    pub fn load_sound(path: &str, should_loop: bool) -> Result<Sound, Box<dyn Error>> {
        let data = StaticSoundData::from_file(path);

        let sound = Sound {
            sound_data: data?,
            loops: should_loop,
        };

        Ok(sound)
    }
}
