use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AppLanguage {
    ChineseSimplified = 0,
    English = 1,
    French = 2,
    Russian = 3,
    Spanish = 4,
}

impl AppLanguage {
    pub const ALL: [Self; 5] = [
        Self::ChineseSimplified,
        Self::English,
        Self::French,
        Self::Russian,
        Self::Spanish,
    ];
}

impl TryFrom<u8> for AppLanguage {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::ChineseSimplified),
            1 => Ok(Self::English),
            2 => Ok(Self::French),
            3 => Ok(Self::Russian),
            4 => Ok(Self::Spanish),
            _ => Err(()),
        }
    }
}

impl Serialize for AppLanguage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for AppLanguage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_from(u8::deserialize(deserializer)?)
            .map_err(|_| serde::de::Error::custom("unsupported application language"))
    }
}
