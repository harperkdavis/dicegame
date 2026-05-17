use std::{
    borrow::Borrow,
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    ops::Index,
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
};

use raylib::prelude::*;
use rust_embed::Embed;
use smartstring::{LazyCompact, SmartString};

pub type ResourceId = SmartString<LazyCompact>;

fn res_id(file_path: &str) -> String {
    let mut path_buf = PathBuf::from(file_path);
    path_buf.set_extension("");
    path_buf.to_string_lossy().to_string()
}

fn get_extension_for_raylib(file_path: &str) -> String {
    Path::new(file_path)
        .extension()
        .and_then(|a| a.to_str())
        .map_or_else(String::new, |ext| format!(".{ext}"))
}

pub trait Resource<'a>: Sized {
    type Context;
    type Asset: Embed;

    fn id(file_path: &str) -> ResourceId {
        SmartString::from_str(&res_id(file_path)).unwrap()
    }

    fn load(
        rl: &mut RaylibHandle,
        ctx: &'a Self::Context,
        file_path: &str,
        data: &[u8],
    ) -> eyre::Result<Self>;
}

pub struct Library<R>(HashMap<ResourceId, R>);

impl<'a, R: Resource<'a>> Library<R> {
    fn load_one(rl: &mut RaylibHandle, ctx: &'a R::Context, file_path: &str) -> eyre::Result<R> {
        let file = R::Asset::get(file_path)
            .ok_or_else(|| eyre::eyre!("fatal: cannot load asset at {file_path}: invalid path"))?;
        R::load(rl, ctx, file_path, &file.data)
    }

    pub fn load(rl: &mut RaylibHandle, ctx: &'a R::Context) -> eyre::Result<Self> {
        let inner: Result<_, _> = R::Asset::iter()
            .map(|file_path| {
                Self::load_one(rl, ctx, &file_path).map(|res| (R::id(&file_path), res))
            })
            .collect();

        inner.map(|inner| Self(inner))
    }

    fn get<K>(&self, id: &K) -> &R
    where
        K: Eq + Hash + Debug + ?Sized,
        ResourceId: Borrow<K>,
    {
        self.0
            .get(id)
            .ok_or_else(|| eyre::eyre!("fatal: invalid asset key {id:?}"))
            .unwrap()
    }
}

impl<'a, R: Resource<'a>, K> Index<&K> for Library<R>
where
    K: Eq + Hash + Debug + ?Sized,
    ResourceId: Borrow<K>,
{
    type Output = R;

    fn index(&self, index: &K) -> &Self::Output {
        self.get(index)
    }
}

#[derive(Embed)]
#[folder = "res/textures"]
pub struct TextureAsset;

impl Resource<'_> for Texture2D {
    type Context = RaylibThread;
    type Asset = TextureAsset;

    fn load(
        rl: &mut RaylibHandle,
        rt: &Self::Context,
        file_path: &str,
        data: &[u8],
    ) -> eyre::Result<Self> {
        rl.load_texture_from_image(
            rt,
            &Image::load_image_from_mem(&get_extension_for_raylib(file_path), data)
                .map_err(|e| eyre::eyre!("could not load image data at {file_path:?}: {e}"))?,
        )
        .map_err(|e| eyre::eyre!("could not load texture from image data at {file_path:?}: {e}"))
    }
}

#[derive(Embed)]
#[folder = "res/sounds"]
pub struct SoundAsset;

impl<'audio> Resource<'audio> for Sound<'audio> {
    type Context = RaylibAudio;
    type Asset = SoundAsset;

    fn load(
        _: &mut RaylibHandle,
        ra: &'audio Self::Context,
        file_path: &str,
        data: &[u8],
    ) -> eyre::Result<Self> {
        ra.new_sound_from_wave(
            &ra.new_wave_from_memory(&get_extension_for_raylib(file_path), data)
                .map_err(move |e| eyre::eyre!("could not load wave data at {file_path:?}: {e}"))?,
        )
        .map_err(move |e| eyre::eyre!("could not load sound from wave data at {file_path:?}: {e}"))
    }
}

#[derive(Embed)]
#[folder = "res/music"]
pub struct MusicAsset;

pub struct MusicData {
    ext: String,
    raw: Vec<u8>,
}

impl Resource<'static> for MusicData {
    type Context = ();
    type Asset = MusicAsset;

    fn load(
        _: &mut RaylibHandle,
        _: &Self::Context,
        file_path: &str,
        data: &[u8],
    ) -> eyre::Result<Self> {
        Ok(Self {
            ext: get_extension_for_raylib(file_path),
            raw: data.to_vec(),
        })
    }
}

#[derive(Embed)]
#[folder = "res/fonts"]
pub struct FontAsset;

fn get_font_size_for_path(file_path: &str) -> Option<i32> {
    let path = Path::new(file_path);
    let stem = path.file_stem()?.to_str()?;
    let (_, suffix) = stem.rsplit_once('_')?;
    let size = suffix.parse().ok()?;
    Some(size)
}

impl Resource<'_> for Font {
    type Context = RaylibThread;
    type Asset = FontAsset;

    fn load(
        rl: &mut RaylibHandle,
        rt: &'_ Self::Context,
        file_path: &str,
        data: &[u8],
    ) -> eyre::Result<Self> {
        rl.load_font_from_memory(
            rt,
            &get_extension_for_raylib(file_path),
            data,
            get_font_size_for_path(file_path).unwrap_or(16),
            None,
        )
        .map_err(|e| eyre::eyre!("failed to load font: {e}"))
    }
}

#[derive(Embed)]
#[folder = "res/shaders"]
pub struct ShaderAsset;

pub type ShaderPtr = Rc<RefCell<Shader>>;

impl Resource<'_> for ShaderPtr {
    type Context = RaylibThread;
    type Asset = ShaderAsset;

    fn load(
        rl: &mut RaylibHandle,
        rt: &'_ Self::Context,
        _: &str,
        data: &[u8],
    ) -> eyre::Result<Self> {
        Ok(Rc::new(RefCell::new(rl.load_shader_from_memory(
            rt,
            None,
            Some(&String::from_utf8_lossy(data)),
        ))))
    }
}

pub struct Res<'audio> {
    tex: Library<Texture2D>,
    snd: Library<Sound<'audio>>,
    mus: Library<MusicData>,
    fnt: Library<Font>,
    sha: Library<ShaderPtr>,
}

impl<'audio> Res<'audio> {
    pub fn tex<K>(&self, k: &K) -> &Texture2D
    where
        K: Eq + Hash + Debug + ?Sized,
        ResourceId: Borrow<K>,
    {
        &self.tex[k]
    }

    pub fn snd<K>(&self, k: &K) -> &Sound<'audio>
    where
        K: Eq + Hash + Debug + ?Sized,
        ResourceId: Borrow<K>,
    {
        &self.snd[k]
    }

    pub fn load_mus<'a, K>(&self, k: &K, ra: &'a RaylibAudio) -> Music<'a>
    where
        K: Eq + Hash + Debug + ?Sized,
        ResourceId: Borrow<K>,
    {
        let data = &self.mus[k];
        ra.new_music_from_memory(&data.ext, &data.raw)
            .expect("could not load music")
    }

    pub fn fnt<K>(&self, k: &K) -> &Font
    where
        K: Eq + Hash + Debug + ?Sized,
        ResourceId: Borrow<K>,
    {
        &self.fnt[k]
    }

    pub fn sha<K>(&self, k: &K) -> &ShaderPtr
    where
        K: Eq + Hash + Debug + ?Sized,
        ResourceId: Borrow<K>,
    {
        &self.sha[k]
    }
}

pub fn load<'audio>(
    rl: &mut RaylibHandle,
    rt: &RaylibThread,
    ra: &'audio RaylibAudio,
) -> eyre::Result<Res<'audio>> {
    Ok(Res {
        tex: Library::load(rl, rt).map_err(|e| eyre::eyre!("failure loading textures: {e}"))?,
        snd: Library::load(rl, ra).map_err(|e| eyre::eyre!("failure loading sounds: {e}"))?,
        mus: Library::load(rl, &()).map_err(|e| eyre::eyre!("failure loading music: {e}"))?,
        fnt: Library::load(rl, rt).map_err(|e| eyre::eyre!("failure loading fonts: {e}"))?,
        sha: Library::load(rl, rt).map_err(|e| eyre::eyre!("failure loading shaders: {e}"))?,
    })
}
