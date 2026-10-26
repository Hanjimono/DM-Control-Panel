use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSecrets {
    pub discord_bot_token: String,
}

impl Default for AppSecrets {
    fn default() -> Self {
        Self {
            discord_bot_token: String::new(),
        }
    }
}

impl AppSecrets {
    pub fn is_discord_token_set(&self) -> bool {
        !self.discord_bot_token.trim().is_empty()
    }
}
