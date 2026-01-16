use std::{
    collections::HashMap,
    fs::{self, DirEntry},
    io,
    path::PathBuf,
    str::FromStr,
};

use raylib::prelude::*;

const ASSETS_PATH: &str = "assets";
const TEXTURES_PATH: &str = "textures";
const SOUNDS_PATH: &str = "sounds";

pub struct Assets<'a> {
    textures: HashMap<String, Texture2D>,
    sounds: HashMap<String, Sound<'a>>,
}

fn load_texture(
    rl: &mut RaylibHandle,
    rt: &RaylibThread,
    entry: Result<DirEntry, io::Error>,
) -> eyre::Result<(String, Texture2D)> {
    let entry = entry.map_err(|e| eyre::eyre!("failed to read entry: {e}"))?;
    let path = entry.path();
    let file_name = path
        .file_stem()
        .ok_or_else(|| eyre::eyre!("failed to resolve file name"))?
        .to_string_lossy()
        .to_string();

    rl.load_texture(
        rt,
        path.to_str()
            .ok_or_else(|| eyre::eyre!("failed to resolve path for {path:?}"))?,
    )
    .map_err(|e| eyre::eyre!("failed to load texture at {path:?}: {e}"))
    .map(|tex| (file_name, tex))
}

fn load_sound<'a>(
    ra: &'a RaylibAudio,
    entry: Result<DirEntry, io::Error>,
) -> eyre::Result<(String, Sound<'a>)> {
    let entry = entry.map_err(|e| eyre::eyre!("failed to read entry: {e}"))?;
    let path = entry.path();
    let file_name = path
        .file_stem()
        .ok_or_else(|| eyre::eyre!("failed to resolve file name"))?
        .to_string_lossy()
        .to_string();

    ra.new_sound(
        path.to_str()
            .ok_or_else(|| eyre::eyre!("failed to resolve path for {path:?}"))?,
    )
    .map_err(|e| eyre::eyre!("failed to load texture at {path:?}: {e}"))
    .map(|tex| (file_name, tex))
}

impl<'assets> Assets<'assets> {
    fn load_textures(
        rl: &mut RaylibHandle,
        rt: &RaylibThread,
    ) -> eyre::Result<HashMap<String, Texture2D>> {
        let mut path = PathBuf::from_str(ASSETS_PATH).unwrap();
        path.push(TEXTURES_PATH);
        let read_dir = fs::read_dir(path.clone())
            .map_err(|e| eyre::eyre!("failed to read path {path:?}: {e}"))?;

        read_dir
            .into_iter()
            .map(|entry| load_texture(rl, rt, entry))
            .collect::<eyre::Result<HashMap<_, _>>>()
    }

    fn load_sounds<'a>(ra: &'a RaylibAudio) -> eyre::Result<HashMap<String, Sound<'a>>> {
        let mut path = PathBuf::from_str(ASSETS_PATH).unwrap();
        path.push(SOUNDS_PATH);
        let read_dir = fs::read_dir(path.clone())
            .map_err(|e| eyre::eyre!("failed to read path {path:?}: {e}"))?;

        read_dir
            .into_iter()
            .map(|entry| load_sound(ra, entry))
            .collect::<eyre::Result<HashMap<_, _>>>()
    }
    pub fn load<'a>(
        rl: &'a mut RaylibHandle,
        rt: &'a RaylibThread,
        ra: &'assets RaylibAudio,
    ) -> eyre::Result<Self> {
        Ok(Self {
            textures: Self::load_textures(rl, rt)
                .map_err(|e| eyre::eyre!("error loading textures: {e}"))?,
            sounds: Self::load_sounds(ra).map_err(|e| eyre::eyre!("error loading sounds: {e}"))?,
        })
    }

    pub fn get_texture(&self, tex: &str) -> &Texture2D {
        assert!(
            self.textures.contains_key(tex),
            "should have texture key: {tex}"
        );
        self.textures.get(tex).unwrap()
    }

    pub fn get_sound(&self, tex: &str) -> &Sound<'assets> {
        assert!(
            self.sounds.contains_key(tex),
            "should have sound key: {tex}"
        );
        self.sounds.get(tex).unwrap()
    }
}
