/*!
Sentence embedder using ONNX Runtime (multilingual-e5-small, 384-dim).

Model files are auto-downloaded from HuggingFace on first use (~118MB cached to
`~/.cache/ocds/models/multilingual-e5-small/`).

This model requires a `"query: "` or `"passage: "` prefix on all inputs:
- **Passage**: tender chunks being indexed
- **Query**: search queries and company profile descriptions being matched against tenders
*/

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;
use tokio::task::JoinError;
use tracing::{info, instrument, warn};

#[derive(Error, Debug)]
pub enum EmbedderError {
    #[error("Model error: {0}")]
    Model(String),

    #[error("Task join error: {0}")]
    Join(#[from] JoinError),
}

const MODEL_ID: &str = "intfloat/multilingual-e5-small";
const MODEL_URL: &str = "https://huggingface.co/intfloat/multilingual-e5-small/resolve/main/onnx/model.onnx";
const TOKENIZER_URL: &str = "https://huggingface.co/intfloat/multilingual-e5-small/resolve/main/tokenizer.json";
pub const EMBEDDING_DIM: usize = 384;
const MAX_LENGTH: usize = 512;

/// Whether the text being embedded is a query (search/profile) or a passage (tender chunk).
/// multilingual-e5-small requires a `"query: "` or `"passage: "` prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextType {
    /// Search queries and company profile descriptions.
    Query,
    /// Tender chunks being indexed.
    Passage,
}

#[derive(Clone)]
pub struct SentenceEmbedder {
    session: Arc<std::sync::Mutex<ort::session::Session>>,
    tokenizer: Arc<tokenizers::Tokenizer>,
}

fn cache_dir() -> Result<PathBuf, EmbedderError> {
    let home = std::env::var("HOME")
        .map_err(|_| EmbedderError::Model("HOME environment variable not set".into()))?;
    Ok(PathBuf::from(home).join(".cache/ocds/models/multilingual-e5-small"))
}

const MAX_RETRIES: u32 = 3;

/// Validate a cached file. Returns true if the file looks intact.
fn validate_cached_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "onnx" => {
            // model.onnx should be >1MB (actual is ~118MB)
            match std::fs::metadata(path) {
                Ok(m) => m.len() > 1_000_000,
                Err(_) => false,
            }
        }
        "json" => {
            // tokenizer.json must be valid JSON
            match std::fs::read(path) {
                Ok(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes).is_ok(),
                Err(_) => false,
            }
        }
        _ => true,
    }
}

/// Returns true if the error is likely transient and worth retrying.
fn is_retryable(err: &EmbedderError) -> bool {
    match err {
        EmbedderError::Model(msg) => {
            // HTTP 5xx
            msg.contains("HTTP 5")
                || msg.contains("Download failed")
                || msg.contains("Failed to read response")
        }
        _ => false,
    }
}

/// Download a file from `url` to `path` atomically (write to .tmp then rename).
async fn download_file(
    client: &reqwest::Client,
    url: &str,
    path: &Path,
) -> Result<(), EmbedderError> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| EmbedderError::Model(format!("Download failed for {url}: {e}")))?;
    if !response.status().is_success() {
        return Err(EmbedderError::Model(format!(
            "Download failed for {url}: HTTP {}",
            response.status()
        )));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| EmbedderError::Model(format!("Failed to read response for {url}: {e}")))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| EmbedderError::Model(format!("Failed to create cache dir: {e}")))?;
    }
    // Atomic write: write to .tmp then rename
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, &bytes)
        .map_err(|e| EmbedderError::Model(format!("Failed to write {}: {e}", tmp_path.display())))?;
    std::fs::rename(&tmp_path, path)
        .map_err(|e| EmbedderError::Model(format!("Failed to rename {}: {e}", tmp_path.display())))?;
    info!("Saved {} ({} bytes)", path.display(), bytes.len());
    Ok(())
}

async fn ensure_file(client: &reqwest::Client, url: &str, path: &Path) -> Result<(), EmbedderError> {
    // Validate cached file if it exists
    if path.exists() {
        if validate_cached_file(path) {
            return Ok(());
        }
        warn!(
            "Cached file {} is corrupt, deleting and re-downloading",
            path.display()
        );
        let _ = std::fs::remove_file(path);
    }

    info!("Downloading {} ...", url);

    // Retry with exponential backoff
    for attempt in 0..=MAX_RETRIES {
        match download_file(client, url, path).await {
            Ok(()) => return Ok(()),
            Err(e) if attempt < MAX_RETRIES && is_retryable(&e) => {
                let delay = Duration::from_secs(1 << attempt); // 1s, 2s, 4s
                warn!(
                    "Download attempt {} failed: {e}. Retrying in {delay:?}...",
                    attempt + 1
                );
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}

impl SentenceEmbedder {
    #[instrument]
    pub async fn new() -> Result<Self, EmbedderError> {
        info!("Loading sentence embedding model ({MODEL_ID}, ort/ONNX) ...");

        let dir = cache_dir()?;
        let model_path = dir.join("model.onnx");
        let tokenizer_path = dir.join("tokenizer.json");

        // Shared HTTP client for all downloads
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(600))
            .build()
            .map_err(|e| EmbedderError::Model(format!("HTTP client: {e}")))?;

        // Download model files if not cached (with validation + retry)
        ensure_file(&client, MODEL_URL, &model_path).await?;
        ensure_file(&client, TOKENIZER_URL, &tokenizer_path).await?;

        // Load session and tokenizer on a blocking thread
        let (session, tokenizer) = tokio::task::spawn_blocking(move || {
            // Cache the optimized graph so ONNX Runtime only optimizes once
            let optimized_path = model_path.with_extension("optimized.onnx");

            let intra_threads = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);

            let session = ort::session::Session::builder()
                .map_err(|e| EmbedderError::Model(format!("Session builder: {e}")))?
                .with_optimization_level(
                    ort::session::builder::GraphOptimizationLevel::Level3,
                )
                .map_err(|e| EmbedderError::Model(format!("Optimization level: {e}")))?
                .with_optimized_model_path(&optimized_path)
                .map_err(|e| EmbedderError::Model(format!("Optimized model path: {e}")))?
                .with_intra_threads(intra_threads)
                .map_err(|e| EmbedderError::Model(format!("Intra threads: {e}")))?
                .commit_from_file(&model_path)
                .map_err(|e| EmbedderError::Model(format!("Load ONNX model: {e}")))?;

            let mut tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
                .map_err(|e| EmbedderError::Model(format!("Load tokenizer: {e}")))?;

            // Pad to longest in batch so all sequences have the same length
            tokenizer.with_padding(Some(tokenizers::PaddingParams::default()));
            tokenizer
                .with_truncation(Some(tokenizers::TruncationParams {
                    max_length: MAX_LENGTH,
                    ..Default::default()
                }))
                .map_err(|e| EmbedderError::Model(format!("Truncation config: {e}")))?;

            Ok::<_, EmbedderError>((session, tokenizer))
        })
        .await??;

        info!("Model loaded successfully (ort/ONNX)");

        Ok(Self {
            session: Arc::new(std::sync::Mutex::new(session)),
            tokenizer: Arc::new(tokenizer),
        })
    }

    #[instrument(skip(self, texts), fields(count = texts.len()))]
    pub async fn embed_batch(
        &self,
        texts: Vec<String>,
        text_type: TextType,
    ) -> Result<Vec<Vec<f32>>, EmbedderError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let session = Arc::clone(&self.session);
        let tokenizer = Arc::clone(&self.tokenizer);

        tokio::task::spawn_blocking(move || {
            // Prepend e5 prefix
            let prefix = match text_type {
                TextType::Query => "query: ",
                TextType::Passage => "passage: ",
            };
            let prefixed: Vec<String> = texts
                .iter()
                .map(|t| format!("{prefix}{t}"))
                .collect();

            // Tokenize
            let encodings = tokenizer
                .encode_batch(
                    prefixed.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                    true,
                )
                .map_err(|e| EmbedderError::Model(format!("Tokenization: {e}")))?;

            let batch_size = encodings.len();
            let seq_len = encodings[0].len();

            // Flatten to [batch * seq_len] for tensor creation
            let ids: Vec<i64> = encodings
                .iter()
                .flat_map(|e| e.get_ids().iter().map(|&i| i as i64))
                .collect();
            let mask: Vec<i64> = encodings
                .iter()
                .flat_map(|e| e.get_attention_mask().iter().map(|&i| i as i64))
                .collect();
            // token_type_ids: all zeros for single-sentence encoding
            let token_type_ids: Vec<i64> = vec![0i64; batch_size * seq_len];

            // Create input tensors [batch, seq_len]
            let ids_tensor = ort::value::TensorRef::from_array_view((
                [batch_size, seq_len],
                &*ids,
            ))
            .map_err(|e| EmbedderError::Model(format!("ids tensor: {e}")))?;
            let mask_tensor = ort::value::TensorRef::from_array_view((
                [batch_size, seq_len],
                &*mask,
            ))
            .map_err(|e| EmbedderError::Model(format!("mask tensor: {e}")))?;
            let type_ids_tensor = ort::value::TensorRef::from_array_view((
                [batch_size, seq_len],
                &*token_type_ids,
            ))
            .map_err(|e| EmbedderError::Model(format!("token_type_ids tensor: {e}")))?;

            // Run inference
            let mut session = session
                .lock()
                .map_err(|e| EmbedderError::Model(format!("Session lock: {e}")))?;
            let outputs = session
                .run(ort::inputs![ids_tensor, mask_tensor, type_ids_tensor])
                .map_err(|e| EmbedderError::Model(format!("Inference: {e}")))?;

            // Extract output
            let hidden = outputs[0]
                .try_extract_array::<f32>()
                .map_err(|e| EmbedderError::Model(format!("Extract output: {e}")))?;
            let shape = hidden.shape();

            let embeddings = if shape.len() == 3 {
                // [batch, seq, dim] — apply mean pooling + L2 normalize
                let dim = shape[2];
                if dim != EMBEDDING_DIM {
                    return Err(EmbedderError::Model(format!(
                        "Expected dimension {EMBEDDING_DIM}, got {dim}"
                    )));
                }
                let data = hidden
                    .as_slice()
                    .ok_or_else(|| EmbedderError::Model("Non-contiguous output".into()))?;
                mean_pool_and_normalize(data, &mask, batch_size, seq_len, dim)
            } else if shape.len() == 2 {
                // [batch, dim] — already pooled, just normalize
                let dim = shape[1];
                let data = hidden
                    .as_slice()
                    .ok_or_else(|| EmbedderError::Model("Non-contiguous output".into()))?;
                let mut result = Vec::with_capacity(batch_size);
                for i in 0..batch_size {
                    let start = i * dim;
                    let mut vec: Vec<f32> = data[start..start + dim].to_vec();
                    l2_normalize(&mut vec);
                    result.push(vec);
                }
                result
            } else {
                return Err(EmbedderError::Model(format!(
                    "Unexpected output shape: {shape:?}"
                )));
            };

            Ok(embeddings)
        })
        .await?
    }

    /// Embed a single text, returning the embedding vector.
    pub async fn embed_text(&self, text: &str, text_type: TextType) -> Result<Vec<f32>, EmbedderError> {
        let mut vecs = self.embed_batch(vec![text.to_owned()], text_type).await?;
        vecs.pop()
            .ok_or_else(|| EmbedderError::Model("Embedder returned no vectors".into()))
    }

    /// Synchronous single-text embed. Suitable for calling inside `spawn_blocking`.
    /// Does NOT spawn a blocking task internally (unlike `embed_text`).
    pub fn embed_text_sync(&self, text: &str, text_type: TextType) -> Result<Vec<f32>, EmbedderError> {
        let prefix = match text_type {
            TextType::Query => "query: ",
            TextType::Passage => "passage: ",
        };
        let prefixed = format!("{prefix}{text}");

        let encodings = self.tokenizer
            .encode_batch(vec![prefixed.as_str()], true)
            .map_err(|e| EmbedderError::Model(format!("Tokenization: {e}")))?;

        let batch_size = encodings.len();
        let seq_len = encodings[0].len();

        let ids: Vec<i64> = encodings.iter()
            .flat_map(|e| e.get_ids().iter().map(|&i| i as i64))
            .collect();
        let mask: Vec<i64> = encodings.iter()
            .flat_map(|e| e.get_attention_mask().iter().map(|&i| i as i64))
            .collect();
        let token_type_ids: Vec<i64> = vec![0i64; batch_size * seq_len];

        let ids_tensor = ort::value::TensorRef::from_array_view(([batch_size, seq_len], &*ids))
            .map_err(|e| EmbedderError::Model(format!("ids tensor: {e}")))?;
        let mask_tensor = ort::value::TensorRef::from_array_view(([batch_size, seq_len], &*mask))
            .map_err(|e| EmbedderError::Model(format!("mask tensor: {e}")))?;
        let type_ids_tensor = ort::value::TensorRef::from_array_view(([batch_size, seq_len], &*token_type_ids))
            .map_err(|e| EmbedderError::Model(format!("token_type_ids tensor: {e}")))?;

        let mut session = self.session.lock()
            .map_err(|e| EmbedderError::Model(format!("Session lock: {e}")))?;
        let outputs = session
            .run(ort::inputs![ids_tensor, mask_tensor, type_ids_tensor])
            .map_err(|e| EmbedderError::Model(format!("Inference: {e}")))?;

        let hidden = outputs[0]
            .try_extract_array::<f32>()
            .map_err(|e| EmbedderError::Model(format!("Extract output: {e}")))?;
        let shape = hidden.shape();

        if shape.len() == 3 {
            let dim = shape[2];
            if dim != EMBEDDING_DIM {
                return Err(EmbedderError::Model(format!("Expected dimension {EMBEDDING_DIM}, got {dim}")));
            }
            let data = hidden.as_slice()
                .ok_or_else(|| EmbedderError::Model("Non-contiguous output".into()))?;
            let mut vecs = mean_pool_and_normalize(data, &mask, batch_size, seq_len, dim);
            vecs.pop().ok_or_else(|| EmbedderError::Model("No vectors returned".into()))
        } else if shape.len() == 2 {
            let dim = shape[1];
            let data = hidden.as_slice()
                .ok_or_else(|| EmbedderError::Model("Non-contiguous output".into()))?;
            let mut vec: Vec<f32> = data[..dim].to_vec();
            l2_normalize(&mut vec);
            Ok(vec)
        } else {
            Err(EmbedderError::Model(format!("Unexpected output shape: {shape:?}")))
        }
    }
}

fn mean_pool_and_normalize(
    hidden: &[f32],
    mask: &[i64],
    batch: usize,
    seq: usize,
    dim: usize,
) -> Vec<Vec<f32>> {
    let mut result = Vec::with_capacity(batch);
    for i in 0..batch {
        let mut pooled = vec![0.0f32; dim];
        let mut token_count = 0.0f32;

        for t in 0..seq {
            let m = mask[i * seq + t] as f32;
            if m > 0.0 {
                let offset = i * seq * dim + t * dim;
                for d in 0..dim {
                    pooled[d] += hidden[offset + d] * m;
                }
                token_count += m;
            }
        }

        if token_count > 0.0 {
            for d in 0..dim {
                pooled[d] /= token_count;
            }
        }

        l2_normalize(&mut pooled);
        result.push(pooled);
    }
    result
}

fn l2_normalize(vec: &mut [f32]) {
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in vec.iter_mut() {
            *x /= norm;
        }
    }
}
