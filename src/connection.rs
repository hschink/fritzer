use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SessionInfo {
    #[serde(rename = "SID")]
    pub sid: String,
    #[serde(rename = "Challenge")]
    pub challenge: String,
    // #[serde(rename = "BlockTime")]
    // block_time: u32
    #[serde(rename = "Users")]
    pub users: Users,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Users {
    #[serde(rename = "$value")]
    pub users: Vec<User>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct User {
    #[serde(rename = "$value")]
    pub username: String,
    pub last: Option<i8>,
}
