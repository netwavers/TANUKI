use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, prompt: &str) -> Result<String>;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

pub struct GeminiClient {
    client: Client,
    api_key: String,
    model: String,
}

impl GeminiClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }
}

#[async_trait]
impl LlmProvider for GeminiClient {
    async fn generate(&self, prompt: &str) -> Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let body = serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }]
            }]
        });

        let resp = self.client.post(&url).json(&body).send().await?;
        let json: serde_json::Value = resp.json().await?;

        // 簡略化のため、最初の候補のみを取得
        let text = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Gemini response: {:?}", json))?;

        Ok(text.to_string())
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-embedding-001:embedContent?key={}",
            self.api_key
        );

        let body = serde_json::json!({
            "model": "models/gemini-embedding-001",
            "content": {
                "parts": [{
                    "text": text
                }]
            }
        });

        let resp = self.client.post(&url).json(&body).send().await?;
        let json: serde_json::Value = resp.json().await?;
        
        if json["embedding"]["values"].is_null() {
            anyhow::bail!("Gemini Embedding API returned null values. Response: {:?}", json);
        }

        let embedding: Vec<f32> = serde_json::from_value(json["embedding"]["values"].clone())?;
        Ok(embedding)
    }
}

pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
    keep_alive: String,
}

impl OllamaClient {
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            model,
            keep_alive: "5m".to_string(), // デフォルトは5分間保持（処理中のキャッシュ用）
        }
    }

    // keep_aliveの変更用ビルダー
    pub fn with_keep_alive(mut self, keep_alive: &str) -> Self {
        self.keep_alive = keep_alive.to_string();
        self
    }

    // 明示的なアンロード（VRAMクリーンアップ用）
    pub async fn unload(&self) -> Result<()> {
        let url = format!("{}/api/generate", self.base_url);
        
        // 1. メインモデルのアンロード
        let body_main = serde_json::json!({
            "model": self.model,
            "prompt": "",
            "stream": false,
            "keep_alive": 0 // 即座にアンロード
        });
        let _ = self.client.post(&url).json(&body_main).send().await;

        // 2. Embeddingモデルのアンロード (抜け穴の解消)
        let body_embed = serde_json::json!({
            "model": "nomic-embed-text",
            "prompt": "",
            "stream": false,
            "keep_alive": 0 // 即座にアンロード
        });
        let _ = self.client.post(&url).json(&body_embed).send().await;

        Ok(())
    }
}

#[async_trait]
impl LlmProvider for OllamaClient {
    async fn generate(&self, prompt: &str) -> Result<String> {
        // Ollamaセンター (Dashboard API) 経由でリクエスト
        let url = format!("{}/api/generate", self.base_url);

        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "keep_alive": self.keep_alive // 動的に設定された保持時間を使用
        });

        let resp = self.client.post(&url).json(&body).send().await?;
        let json: serde_json::Value = resp.json().await?;

        let text = json["response"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to parse OllamaCenter response. Raw JSON: {:?}", json))?;

        Ok(text.to_string())
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Ollamaセンター (Dashboard API) 経由でリクエスト
        let url = format!("{}/api/embeddings", self.base_url);

        // コンテキスト長制限を考慮して、非常に長いテキストは安全に切り詰める
        let safe_text = match text.char_indices().nth(2000) {
            Some((idx, _)) => &text[..idx],
            None => text,
        };

        let body = serde_json::json!({
            "model": "nomic-embed-text",
            "prompt": safe_text,
            "keep_alive": self.keep_alive
        });

        let resp = self.client.post(&url).json(&body).send().await?;
        let json: serde_json::Value = resp.json().await?;

        let embedding = json["embedding"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Failed to parse OllamaCenter embedding response: {:?}", json))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect::<Vec<f32>>();

        Ok(embedding)
    }
}
