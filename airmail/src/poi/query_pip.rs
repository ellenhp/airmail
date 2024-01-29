use std::error::Error;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PipAdminArea {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PipResponse {
    pub locality: Option<Vec<PipAdminArea>>,
    pub county: Option<Vec<PipAdminArea>>,
    pub region: Option<Vec<PipAdminArea>>,
    pub country: Option<Vec<PipAdminArea>>,
}

thread_local! {
    static HTTP_CLIENT: reqwest::Client = reqwest::Client::new();
}

pub async fn query_pip(lat: f64, lng: f64) -> Result<PipResponse, Box<dyn Error>> {
    let url = format!("http://localhost:3102/{}/{}", lng, lat);
    let response = HTTP_CLIENT.with(|client| client.get(&url).send()).await?;
    if response.status() != 200 {
        return Err(format!("HTTP error: {}", response.status()).into());
    }
    let response_json = response.text().await?;
    let maybe_response = serde_json::from_str(&response_json);
    Ok(maybe_response?)
}
