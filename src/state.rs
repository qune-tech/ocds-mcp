use std::sync::{Mutex, OnceLock};

use crate::embedder::SentenceEmbedder;

use crate::profile_db::ProfileDb;

pub struct SharedState {
    pub db: Mutex<ProfileDb>,
    pub data_dir: String,
    /// Set once from a background task. `get()` returns `None` while loading.
    pub embedder: OnceLock<SentenceEmbedder>,
    pub api_url: String,
    pub http: reqwest::Client,
    pub api_key: Option<String>,
}

impl std::fmt::Debug for SharedState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedState")
            .field("data_dir", &self.data_dir)
            .field("api_url", &self.api_url)
            .field("embedder_loaded", &self.embedder.get().is_some())
            .finish()
    }
}
