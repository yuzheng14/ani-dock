use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    pub(crate) anime_sn: u32,
    pub(crate) login: bool,
    // promote: [],
    /// unknown value meaning
    pub(crate) r18: u32,
    pub(crate) src: String,
    /// unknown value meaning
    pub(crate) time: u32,
    /// is vip account
    pub(crate) vip: bool,
}

impl Token {
    pub fn login(&self) -> bool {
        self.login
    }

    pub fn vip(&self) -> bool {
        self.vip
    }

    pub fn time(&self) -> u32 {
        self.time
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenError {
    code: String,
    message: String,
}

impl Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[code]: {}, [message]: {}", self.code, self.message)
    }
}

impl std::error::Error for TokenError {}
