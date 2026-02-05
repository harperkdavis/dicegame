use rand::Rng;
use raylib::math::Rectangle;

#[repr(u8)]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    #[default]
    Blank,
    TenPoints,
    Two,
    Three,
    Four,
    FivePoints,
    Six,
    WildSun,
}

impl Face {
    const fn texture_index(&self) -> usize {
        match self {
            Self::Blank | Self::TenPoints => 0,
            Self::Two => 2,
            Self::Three | Self::WildSun => 3,
            Self::Four => 4,
            Self::FivePoints => 1,
            Self::Six => 5,
        }
    }

    pub const fn point_value(&self) -> u32 {
        match self {
            Self::FivePoints => 5,
            Self::TenPoints | Self::WildSun => 10,
            _ => 0,
        }
    }

    pub const fn is_scoring(&self) -> bool {
        self.point_value() > 0
    }

    pub const fn face_value(&self) -> u32 {
        match self {
            Self::Two => 2,
            Self::Three => 3,
            Self::Four => 4,
            Self::FivePoints => 5,
            Self::Six => 6,
            Self::TenPoints => 10,
            _ => 0,
        }
    }

    pub const fn will_supernova(&self) -> bool {
        matches!(self, Self::TenPoints)
    }

    pub fn get_rect(&self, texture_index: usize) -> Rectangle {
        Rectangle::new(
            self.texture_index() as f32 * 32.0,
            texture_index as f32 * 32.0,
            32.0,
            32.0,
        )
    }

    pub const fn is_wild(&self) -> bool {
        matches!(self, Self::WildSun)
    }

    pub fn matches(&self, other: &Self) -> bool {
        self.is_wild() || other.is_wild() || self == other
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Die {
    faces: [Face; 6],
    texture_index: usize,
}

impl Die {
    pub const fn new(faces: [Face; 6], texture_index: usize) -> Self {
        Self {
            faces,
            texture_index,
        }
    }

    pub fn roll(&self, rng: &mut impl Rng) -> Face {
        self.faces[rng.random_range(0..6)]
    }

    pub fn face(&self, index: usize) -> Face {
        self.faces[index.rem_euclid(6)]
    }

    pub fn face_rect(&self, index: usize) -> Rectangle {
        self.face(index).get_rect(self.texture_index)
    }

    pub fn texture_index(&self) -> usize {
        self.texture_index
    }
}
