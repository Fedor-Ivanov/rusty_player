//! HTTP for online services: lyrics (LRCLIB), artist images (Deezer).

use std::time::Duration;

/// Services ask the app to identify itself: name, version and link
const USER_AGENT: &str = concat!(
    "LMP ",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/Fedor-Ivanov/rusty_palyer)"
);
const TIMEOUT: Duration = Duration::from_secs(15);

/// A client with shared settings. Status codes are not errors: 404 from the services means
/// "not found", the caller handles it
pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .user_agent(USER_AGENT)
        .http_status_as_error(false)
        .build()
        .into()
}
