use serde::{Deserialize, Serialize};

/// Discord guild and the voice channel used after the app opens.
///
/// Empty strings mean the user has not entered a value yet.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DiscordSettings {
    pub guild_id: String,
    pub voice_channel_id: String,
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
    pub guild_id: Option<String>,
    pub voice_channel_id: Option<String>,
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
        if let Some(guild_id) = discord_patch.guild_id {
            self.discord.guild_id = guild_id.trim().to_string();
        }
        if let Some(voice_channel_id) = discord_patch.voice_channel_id {
            self.discord.voice_channel_id = voice_channel_id.trim().to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_trims_a_guild_id_and_keeps_the_omitted_channel() {
        let mut settings = UserSettings::default();
        settings.discord.voice_channel_id = "99".to_string();
        settings.apply_patch(UserSettingsPatch {
            discord: Some(DiscordSettingsPatch {
                guild_id: Some("  42  ".to_string()),
                voice_channel_id: None,
            }),
        });
        assert_eq!(settings.discord.guild_id, "42");
        assert_eq!(settings.discord.voice_channel_id, "99");
    }

    #[test]
    fn old_ui_json_loads_without_discord_ids() {
        let settings: UserSettings =
            serde_json::from_str(r#"{"ui":{"hideSongName":true,"songCardLayout":"compact"}}"#)
                .expect("old settings json");
        assert_eq!(settings.discord.guild_id, "");
        assert_eq!(settings.discord.voice_channel_id, "");

        let saved = serde_json::to_string(&settings).expect("serialize settings");
        assert!(!saved.contains("\"ui\""));
        assert!(saved.contains("guildId"));
        assert!(saved.contains("voiceChannelId"));
    }

    #[test]
    fn patch_rejects_the_removed_ui_branch() {
        let error = serde_json::from_str::<UserSettingsPatch>(r#"{"ui":{}}"#);
        assert!(error.is_err());
    }
}
