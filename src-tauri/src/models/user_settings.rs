use serde::{Deserialize, Serialize};

/// Discord server and the default voice channel used after the app opens.
///
/// Empty strings mean the user has not entered a value yet.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DiscordSettings {
    pub server_id: String,
    pub default_voice_channel_id: String,
}

/// Typed user settings document. Secrets are stored separately.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UserSettings {
    pub discord: DiscordSettings,
}

/// Partial Discord settings. Missing fields stay unchanged.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct DiscordSettingsPatch {
    pub server_id: Option<String>,
    pub default_voice_channel_id: Option<String>,
}

/// Partial user settings. Missing branches stay unchanged.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct UserSettingsPatch {
    pub discord: Option<DiscordSettingsPatch>,
}

impl UserSettings {
    /// Applies a patch. Omitted fields are left as they are. Passed strings are trimmed.
    pub fn apply_patch(&mut self, patch: UserSettingsPatch) {
        let Some(discord_patch) = patch.discord else {
            return;
        };
        if let Some(server_id) = discord_patch.server_id {
            self.discord.server_id = server_id.trim().to_string();
        }
        if let Some(default_voice_channel_id) = discord_patch.default_voice_channel_id {
            self.discord.default_voice_channel_id = default_voice_channel_id.trim().to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_trims_a_server_id_and_keeps_the_omitted_channel() {
        let mut settings = UserSettings::default();
        settings.discord.default_voice_channel_id = "99".to_string();
        settings.apply_patch(UserSettingsPatch {
            discord: Some(DiscordSettingsPatch {
                server_id: Some("  42  ".to_string()),
                default_voice_channel_id: None,
            }),
        });
        assert_eq!(settings.discord.server_id, "42");
        assert_eq!(settings.discord.default_voice_channel_id, "99");
    }

    #[test]
    fn old_ui_json_loads_without_discord_ids() {
        let settings: UserSettings =
            serde_json::from_str(r#"{"ui":{"hideSongName":true,"songCardLayout":"compact"}}"#)
                .expect("old settings json");
        assert_eq!(settings.discord.server_id, "");
        assert_eq!(settings.discord.default_voice_channel_id, "");

        let saved = serde_json::to_string(&settings).expect("serialize settings");
        assert!(!saved.contains("\"ui\""));
        assert!(saved.contains("serverId"));
        assert!(saved.contains("defaultVoiceChannelId"));
    }

    #[test]
    fn patch_rejects_the_removed_ui_branch() {
        let error = serde_json::from_str::<UserSettingsPatch>(r#"{"ui":{}}"#);
        assert!(error.is_err());
    }
}
