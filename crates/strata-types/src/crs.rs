use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WorkCrs {
    Epsg5186,
    #[default]
    Epsg5187,
}

impl WorkCrs {
    pub const fn epsg_code(self) -> u32 {
        match self {
            Self::Epsg5186 => 5186,
            Self::Epsg5187 => 5187,
        }
    }

    pub const fn auth_id(self) -> &'static str {
        match self {
            Self::Epsg5186 => "EPSG:5186",
            Self::Epsg5187 => "EPSG:5187",
        }
    }

    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::Epsg5186 => "EPSG:5186 (중부원점)",
            Self::Epsg5187 => "EPSG:5187 (동부원점)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SubmitCrs {
    #[default]
    Epsg5179,
}

impl SubmitCrs {
    pub const fn epsg_code(self) -> u32 {
        5179
    }

    pub const fn auth_id(self) -> &'static str {
        "EPSG:5179"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CrsCode {
    Epsg5187,
    Epsg5186,
    Epsg5179,
    Epsg5174,
    Epsg3857,
    Epsg4326,
    Epsg900913,
}

impl CrsCode {
    pub const fn auth_id(self) -> &'static str {
        match self {
            Self::Epsg5187 => "EPSG:5187",
            Self::Epsg5186 => "EPSG:5186",
            Self::Epsg5179 => "EPSG:5179",
            Self::Epsg5174 => "EPSG:5174",
            Self::Epsg3857 => "EPSG:3857",
            Self::Epsg4326 => "EPSG:4326",
            Self::Epsg900913 => "EPSG:900913",
        }
    }

    pub const fn epsg_number(self) -> u32 {
        match self {
            Self::Epsg5187 => 5187,
            Self::Epsg5186 => 5186,
            Self::Epsg5179 => 5179,
            Self::Epsg5174 => 5174,
            Self::Epsg3857 => 3857,
            Self::Epsg4326 => 4326,
            Self::Epsg900913 => 900913,
        }
    }
}

impl From<WorkCrs> for CrsCode {
    fn from(w: WorkCrs) -> Self {
        match w {
            WorkCrs::Epsg5186 => Self::Epsg5186,
            WorkCrs::Epsg5187 => Self::Epsg5187,
        }
    }
}
