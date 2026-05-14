//! `hackline login --server URL --token TOKEN [--name NAME]`.
//! For the very first call against a fresh gateway, this is the
//! claim flow; subsequent calls just cache credentials.

use crate::client::Client;
use crate::config::{self, Credentials};

pub async fn run(server: &str, token: &str, name: &str) -> anyhow::Result<()> {
    let c = Client::new(server, "");
    let body = serde_json::json!({ "token": token, "name": name });
    let resp = c.post_no_auth("/v1/claim", &body).await?;

    let user_id = resp["user_id"].as_i64().unwrap_or(0);
    let bearer = resp["token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("no token in response"))?;

    config::save_credentials(&Credentials {
        server: server.to_string(),
        token: bearer.to_string(),
        user_id,
        name: name.to_string(),
    })?;

    println!("Logged in as {name} (user_id={user_id})");
    println!("Credentials cached.");
    Ok(())
}
