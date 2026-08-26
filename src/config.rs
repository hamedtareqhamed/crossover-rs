use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CrosshairStyle {
    Cross,
    Dot,
    Circle,
    CircleDot,
    Chevron,
    Box,
    TStyle,
    Svg(String),
}

impl CrosshairStyle {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cross" => CrosshairStyle::Cross,
            "dot" => CrosshairStyle::Dot,
            "circle" => CrosshairStyle::Circle,
            "circledot" => CrosshairStyle::CircleDot,
            "chevron" => CrosshairStyle::Chevron,
            "box" => CrosshairStyle::Box,
            "tstyle" | "t-style" => CrosshairStyle::TStyle,
            other => CrosshairStyle::Svg(other.to_string()),
        }
    }

    pub fn to_string_repr(&self) -> String {
        match self {
            CrosshairStyle::Cross => "Cross".to_string(),
            CrosshairStyle::Dot => "Dot".to_string(),
            CrosshairStyle::Circle => "Circle".to_string(),
            CrosshairStyle::CircleDot => "CircleDot".to_string(),
            CrosshairStyle::Chevron => "Chevron".to_string(),
            CrosshairStyle::Box => "Box".to_string(),
            CrosshairStyle::TStyle => "TStyle".to_string(),
            CrosshairStyle::Svg(name) => format!("Svg({})", name),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub style: CrosshairStyle,
    pub size: u32,
    pub thickness: u32,
    pub gap: u32,
    pub color: [u8; 4], // RGBA (0-255)
    pub outline: bool,
    pub outline_color: [u8; 4],
    pub outline_thickness: u32,
    pub dot: bool,
    pub dot_size: u32,
    pub dot_color: [u8; 4],
    pub opacity: f32, // 0.0 to 1.0
    pub offset_x: i32,
    pub offset_y: i32,
    pub visible: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            style: CrosshairStyle::Cross,
            size: 32,
            thickness: 2,
            gap: 5,
            color: [0, 255, 136, 255], // Neon Green
            outline: true,
            outline_color: [0, 0, 0, 255], // Black
            outline_thickness: 1,
            dot: true,
            dot_size: 2,
            dot_color: [0, 255, 136, 255],
            opacity: 1.0,
            offset_x: 0,
            offset_y: 0,
            visible: true,
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let mut path = PathBuf::from(home);
        path.push(".config");
        path.push("crossover");
        path.push("config.toml");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cfg) = toml::from_str(&content) {
                return cfg;
            }
        }
        let cfg = Self::default();
        let _ = cfg.save();
        cfg
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let content = toml::to_string_pretty(self).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, content)
    }
}
