use std::convert::TryFrom;

pub enum KeyValueEncoding {
    Utf8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatType {
    Invalid = 0,
    Integer = 1,
    Float = 2,
    AverageRate = 3,
    Achievements = 4,
    GroupAchievements = 5,
}

impl TryFrom<u8> for UserStatType {
    type Error = String;
    
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(UserStatType::Invalid),
            1 => Ok(UserStatType::Integer),
            2 => Ok(UserStatType::Float),
            3 => Ok(UserStatType::AverageRate),
            4 => Ok(UserStatType::Achievements),
            5 => Ok(UserStatType::GroupAchievements),
            _ => Err(format!("Invalid UserStatType value: {}", value)),
        }
    }
}

impl From<UserStatType> for u8 {
    fn from(stat_type: UserStatType) -> u8 {
        stat_type as u8
    }
}