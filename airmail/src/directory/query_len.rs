use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::Duration,
};

use log::{error, info};

use crate::directory::BLOCKING_HTTP_CLIENT;

static LENGTHS: OnceLock<Mutex<HashMap<PathBuf, usize>>> = OnceLock::new();

pub(crate) fn len(url: &str) -> usize {
    let lengths = LENGTHS.get_or_init(|| Mutex::new(HashMap::new()));
    {
        let lengths = lengths.lock().unwrap();
        if let Some(length) = lengths.get(&PathBuf::from(url)) {
            return *length;
        }
    }

    info!("Fetching length from: {}", url);
    let response = BLOCKING_HTTP_CLIENT
        .with(|client| client.head(url).timeout(Duration::from_millis(500)).send());
    if let Err(e) = response {
        error!("Error fetching length: {:?}", e);
        panic!();
    }
    let response = response.unwrap();
    if response.status() != 200 {
        error!("Response: {:?}", response);
        panic!();
    } else {
        let length = response
            .headers()
            .get("Content-Length")
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        info!("Length: {}", length);
        let mut lengths = lengths.lock().unwrap();
        lengths.insert(PathBuf::from(url), length);
        length
    }
}
