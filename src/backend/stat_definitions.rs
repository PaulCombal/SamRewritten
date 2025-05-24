use std::fmt;
use std::time::SystemTime;
use std::ops::{BitOr, BitOrAssign};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatFlags {
    bits: u32,
}

impl StatFlags {
    pub const NONE: StatFlags = StatFlags { bits: 0 };
    pub const INCREMENT_ONLY: StatFlags = StatFlags { bits: 1 << 0 };
    pub const PROTECTED: StatFlags = StatFlags { bits: 1 << 1 };
    pub const UNKNOWN_PERMISSION: StatFlags = StatFlags { bits: 1 << 2 };

    pub fn bits(&self) -> u32 {
        self.bits
    }

    pub fn contains(&self, other: StatFlags) -> bool {
        (self.bits & other.bits) == other.bits
    }

    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }
}

impl BitOr for StatFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        StatFlags { bits: self.bits | rhs.bits }
    }
}

impl BitOrAssign for StatFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.bits |= rhs.bits;
    }
}

// Error type
#[derive(Debug, Clone)]
pub struct StatIsProtectedError {
    message: String,
}

impl StatIsProtectedError {
    pub fn new() -> Self {
        StatIsProtectedError {
            message: "Stat is protected".to_string(),
        }
    }

    pub fn with_message(message: &str) -> Self {
        StatIsProtectedError {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for StatIsProtectedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for StatIsProtectedError {}

// Stat Definition hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatDefinition {
    Float(FloatStatDefinition),
    Integer(IntegerStatDefinition),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseStatDefinition {
    pub id: String,
    pub app_id: u32,
    pub display_name: String,
    pub permission: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatStatDefinition {
    pub base: BaseStatDefinition,
    pub min_value: f32,
    pub max_value: f32,
    pub max_change: f32,
    pub increment_only: bool,
    pub default_value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegerStatDefinition {
    pub base: BaseStatDefinition,
    pub min_value: i32,
    pub max_value: i32,
    pub max_change: i32,
    pub increment_only: bool,
    pub set_by_trusted_game_server: bool,
    pub default_value: i32,
}

// Stat Info hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatInfo {
    Float(FloatStatInfo),
    Integer(IntStatInfo),
}

impl StatInfo {
    pub fn id(&self) -> &str {
        match self {
            StatInfo::Float(f) => &f.id,
            StatInfo::Integer(i) => &i.id,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            StatInfo::Float(f) => &f.display_name,
            StatInfo::Integer(i) => &i.display_name,
        }
    }

    pub fn permission(&self) -> i32 {
        match self {
            StatInfo::Float(f) => f.permission,
            StatInfo::Integer(i) => i.permission,
        }
    }

    pub fn is_modified(&self) -> bool {
        match self {
            StatInfo::Float(f) => f.is_modified(),
            StatInfo::Integer(i) => i.is_modified(),
        }
    }

    pub fn extra(&self) -> StatFlags {
        match self {
            StatInfo::Float(f) => f.extra(),
            StatInfo::Integer(i) => i.extra(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatStatInfo {
    pub id: String,
    pub app_id: u32,
    pub display_name: String,
    pub is_increment_only: bool,
    pub permission: i32,
    pub original_value: f32,
    pub float_value: f32,
}

impl FloatStatInfo {
    pub fn value(&self) -> f32 {
        self.float_value
    }

    pub fn set_value(&mut self, value: f32) -> Result<(), StatIsProtectedError> {
        if (self.permission & 2) != 0 && !self.float_value.eq(&value) {
            return Err(StatIsProtectedError::new());
        }
        self.float_value = value;
        Ok(())
    }

    pub fn is_modified(&self) -> bool {
        !self.float_value.eq(&self.original_value)
    }

    pub fn extra(&self) -> StatFlags {
        let mut flags = StatFlags::NONE;
        if self.is_increment_only {
            flags |= StatFlags::INCREMENT_ONLY;
        }
        if (self.permission & 2) != 0 {
            flags |= StatFlags::PROTECTED;
        }
        if (self.permission & !2) != 0 {
            flags |= StatFlags::UNKNOWN_PERMISSION;
        }
        flags
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntStatInfo {
    pub id: String,
    pub app_id: u32,
    pub display_name: String,
    pub is_increment_only: bool,
    pub permission: i32,
    pub original_value: i32,
    pub int_value: i32,
}

impl IntStatInfo {
    pub fn value(&self) -> i32 {
        self.int_value
    }

    pub fn set_value(&mut self, value: i32) -> Result<(), StatIsProtectedError> {
        if (self.permission & 2) != 0 && self.int_value != value {
            return Err(StatIsProtectedError::new());
        }
        self.int_value = value;
        Ok(())
    }

    pub fn is_modified(&self) -> bool {
        self.int_value != self.original_value
    }

    pub fn extra(&self) -> StatFlags {
        let mut flags = StatFlags::NONE;
        if self.is_increment_only {
            flags |= StatFlags::INCREMENT_ONLY;
        }
        if (self.permission & 2) != 0 {
            flags |= StatFlags::PROTECTED;
        }
        if (self.permission & !2) != 0 {
            flags |= StatFlags::UNKNOWN_PERMISSION;
        }
        flags
    }
}

// Achievement types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementDefinition {
    pub id: String,
    pub app_id: u32,
    pub name: String,
    pub description: String,
    pub icon_normal: String,
    pub icon_locked: String,
    pub is_hidden: bool,
    pub permission: i32,
}

impl fmt::Display for AchievementDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}",
            if self.name.is_empty() { self.id.clone() } else { self.name.clone() },
            self.permission
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementInfo {
    pub id: String,
    pub is_achieved: bool,
    pub unlock_time: Option<SystemTime>,
    pub permission: i32,
    pub icon_normal: String,
    pub icon_locked: String,
    pub name: String,
    pub description: String,
    pub global_achieved_percent: Option<f32>,
}