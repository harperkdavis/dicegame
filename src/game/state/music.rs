use std::collections::HashMap;

use eyre::OptionExt;
use raylib::audio::Music;

use crate::{Str, game::Static, util::lerp};

pub const EMPTY_TRACK: &str = "000_Empty";
pub const BATTLE_TRACK: &str = "003_EVISCERATE!";
pub const BATTLE_WIN_TRACK: &str = "005_EXECUTION!";
pub const BATTLE_LOSE_TRACK: &str = "001_Lucid";

const MAX_MUSIC_VOLUME: f32 = 1.0;
const EPSILON: f32 = 0.0001;

struct MusicInstance<'a> {
    pub music: Music<'a>,
    current_pitch: f32,
    current_volume: f32,
    resume_location: f32,
}

impl<'a> MusicInstance<'a> {
    fn new(music: Music<'a>) -> Self {
        Self {
            music,
            current_pitch: 1.0,
            current_volume: 0.0,
            resume_location: 0.0,
        }
    }
}

pub struct MusicController<'a> {
    music_table: HashMap<Str, MusicInstance<'a>>,
    music_volume: f32,

    battle_music: Music<'a>,
    battle_win_music: Music<'a>,
    battle_lose_music: Music<'a>,

    current_area_music: Str,
    override_music: Option<Str>,

    battle_music_state: Option<Option<bool>>,
}

impl<'a> MusicController<'a> {
    pub fn load(Static { ra, res, .. }: Static<'a>, music_volume: f32) -> eyre::Result<Self> {
        let mut music_table: HashMap<Str, MusicInstance> = HashMap::new();
        let mut battle_music: Option<Music> = None;
        let mut battle_win_music: Option<Music> = None;
        let mut battle_lose_music: Option<Music> = None;
        for (name, _) in res.mus_iter() {
            let mut music = res.load_mus(name, ra);
            music.looping = true;
            music.set_pitch(1.0);
            music.set_volume(music_volume * MAX_MUSIC_VOLUME);
            match name.as_str() {
                BATTLE_TRACK => battle_music = Some(music),
                BATTLE_WIN_TRACK => {
                    music.looping = false;
                    battle_win_music = Some(music);
                }
                BATTLE_LOSE_TRACK => battle_lose_music = Some(music),
                _ => {
                    music_table.insert(name.to_owned(), MusicInstance::new(music));
                }
            }
        }
        Ok(Self {
            music_table,
            music_volume,

            battle_music: battle_music.ok_or_eyre("could not find battle music")?,
            battle_win_music: battle_win_music.ok_or_eyre("could not find battle win music")?,
            battle_lose_music: battle_lose_music.ok_or_eyre("could not find battle lose music")?,

            current_area_music: EMPTY_TRACK.into(),
            override_music: None,

            battle_music_state: None,
        })
    }

    fn vol(&self) -> f32 {
        MAX_MUSIC_VOLUME * self.music_volume
    }

    pub fn update(
        &mut self,
        delta: f32,
        entering_battle: bool,
        battle_music_state: Option<(Option<bool>, f32)>,
    ) {
        let vol = self.vol();
        for (name, instance) in self.music_table.iter_mut() {
            let target_volume = if self.battle_music_state.is_some() || entering_battle {
                0.0
            } else if let Some(track) = self.override_music.as_ref() {
                if track == name { 1.0 } else { 0.0 }
            } else if &self.current_area_music == name {
                1.0
            } else {
                0.0
            };

            let new_volume = lerp(instance.current_volume, target_volume, delta * 0.1 * 60.0);

            instance.music.set_volume(new_volume * vol);
            if instance.current_volume > EPSILON {
                instance.music.update_stream();
                // music is STOPPING
                if new_volume < EPSILON {
                    instance.resume_location = instance.music.get_time_played();
                    instance.music.pause_stream();
                }
            } else if new_volume > EPSILON {
                // music is STARTING
                instance.music.seek_stream(instance.resume_location);
                instance.music.play_stream();
                instance.music.set_volume(0.0);
            }

            instance.current_volume = new_volume;
        }

        if let Some((win_loss, reported_volume)) = battle_music_state {
            if let Some(prev_win_loss) = self.battle_music_state {
                // some music is playing
                match (prev_win_loss, win_loss) {
                    (None, None) => {
                        self.battle_music.update_stream();
                        self.battle_music.set_volume(reported_volume * self.vol());
                    }
                    (None, Some(win)) => {
                        // battle ended this frame.
                        // slightly fade out battle music
                        self.battle_music.set_volume(0.5 * self.vol());
                        let track = if win {
                            &self.battle_win_music
                        } else {
                            &self.battle_lose_music
                        };

                        track.set_volume(reported_volume * self.vol());
                        track.seek_stream(0.0);
                        track.play_stream();
                    }
                    (Some(win), Some(_)) => {
                        if self.battle_music.is_stream_playing() {
                            self.battle_music.set_volume(0.0);
                            self.battle_music.stop_stream();
                        }

                        let track = if win {
                            &self.battle_win_music
                        } else {
                            &self.battle_lose_music
                        };
                        track.set_volume(reported_volume * self.vol());
                        track.update_stream();
                    }
                    _ => (),
                }
            } else {
                // music has not started, play battle track
                println!("starting battle track!");
                self.battle_music.set_volume(reported_volume * self.vol());
                self.battle_music.seek_stream(0.0);
                self.battle_music.play_stream();
            }

            self.battle_music_state = Some(win_loss);
        } else if self.battle_music_state.is_some() {
            self.battle_music.set_volume(0.0);
            self.battle_win_music.set_volume(0.0);
            if self.battle_win_music.is_stream_playing() {
                self.battle_win_music.stop_stream();
            }
            self.battle_lose_music.set_volume(0.0);
            if self.battle_lose_music.is_stream_playing() {
                self.battle_lose_music.stop_stream();
            }
            self.battle_music_state = None;
        }
    }
}
