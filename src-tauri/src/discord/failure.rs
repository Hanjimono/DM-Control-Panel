//! Maps a fatal gateway error onto the English sentence stored in the snapshot.
//!
//! The token itself is never copied into the message. Callers log `Display`, not `Debug`.

use serenity::gateway::GatewayError;
use serenity::Error as SerenityError;

pub(crate) const INVALID_TOKEN_MESSAGE: &str =
    "The Discord bot token is invalid or has been revoked.";

/// Sentence for a fatal `Client::start` error.
pub(crate) fn failure_message(err: &SerenityError) -> String {
    let status = match err {
        SerenityError::Http(http) => http.status_code().map(|code| code.as_u16()),
        _ => None,
    };
    let gateway_invalid = matches!(
        err,
        SerenityError::Gateway(GatewayError::InvalidAuthentication)
    );
    failure_message_for(status, gateway_invalid, &err.to_string())
}

fn failure_message_for(status: Option<u16>, gateway_invalid: bool, fallback: &str) -> String {
    // Songbird registration makes serenity call `get_current_user` before the gateway,
    // so a revoked token usually arrives as HTTP 401 rather than InvalidAuthentication.
    if gateway_invalid || status == Some(401) {
        INVALID_TOKEN_MESSAGE.to_string()
    } else {
        fallback.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthorized_and_invalid_authentication_share_one_sentence() {
        assert_eq!(
            failure_message_for(Some(401), false, "401: Unauthorized"),
            INVALID_TOKEN_MESSAGE
        );
        assert_eq!(
            failure_message_for(None, true, "Sent invalid authentication"),
            INVALID_TOKEN_MESSAGE
        );
    }

    #[test]
    fn other_failures_keep_the_error_text() {
        assert_eq!(
            failure_message_for(Some(500), false, "Error while sending HTTP request."),
            "Error while sending HTTP request."
        );
    }

    #[test]
    fn gateway_invalid_authentication_uses_the_fixed_sentence() {
        let err = SerenityError::Gateway(GatewayError::InvalidAuthentication);
        assert_eq!(failure_message(&err), INVALID_TOKEN_MESSAGE);
    }
}
