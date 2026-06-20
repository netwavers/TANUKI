use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, prompt: &str) -> Result<String>;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn unload(&self) -> Result<()>;
}

// ==========================================
// llm_manager 設定ファイル対応データ構造
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelProvider {
    #[serde(rename = "OpenAI")]
    OpenAi,
    #[serde(rename = "Anthropic")]
    Anthropic,
    #[serde(rename = "Gemini")]
    Gemini,
    #[serde(rename = "OpenRouter")]
    OpenRouter,
    #[serde(rename = "Ollama")]
    Ollama,
    #[serde(rename = "Local")]
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub vision: bool,
    #[serde(default)]
    pub websearch: bool,
    #[serde(default)]
    pub cors: bool,
    #[serde(default)]
    pub curl: bool,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            reasoning: false,
            vision: false,
            websearch: false,
            cors: false,
            curl: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatModelConfig {
    pub model_name: String,
    pub provider: ModelProvider,
    pub display_name: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    #[serde(default)]
    pub capabilities: ModelCapabilities,
    #[serde(default)]
    pub custom_options: serde_json::Value,
}

/// models_config.json から設定を読み込み、対応する LlmProvider を生成する
pub fn load_provider(config_path: &str, model_name: &str) -> Result<Box<dyn LlmProvider>> {
    let mut file = File::open(config_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // オブジェクトマップか配列リストかを柔軟にデシリアライズ
    let config: ChatModelConfig = if let Ok(map) = serde_json::from_str::<HashMap<String, ChatModelConfig>>(&contents) {
        map.get(model_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Model '{}' not found in models config map", model_name))?
    } else if let Ok(list) = serde_json::from_str::<Vec<ChatModelConfig>>(&contents) {
        list.into_iter()
            .find(|m| m.model_name == model_name)
            .ok_or_else(|| anyhow::anyhow!("Model '{}' not found in models config list", model_name))?
    } else {
        anyhow::bail!("Invalid models config format in {}", config_path);
    };

    match config.provider {
        ModelProvider::Gemini => {
            let api_key = config.api_key.ok_or_else(|| anyhow::anyhow!("API key is required for Gemini provider"))?;
            let model = config.model_name;
            Ok(Box::new(GeminiClient::new(api_key, model)))
        }
        ModelProvider::Ollama | ModelProvider::Local => {
            let base_url = config.base_url.unwrap_or_else(|| "http://localhost:11434".to_string());
            // API のパス調整 (/v1などが末尾にある場合はトリムしてOllamaネイティブのベースに合わせる)
            let clean_url = if base_url.ends_url_with_v1() {
                base_url.trim_end_matches("/v1").to_string()
            } else if base_url.ends_url_with_v1_slash() {
                base_url.trim_end_matches("/v1/").to_string()
            } else {
                base_url
            };
            let model = config.model_name;
            Ok(Box::new(OllamaClient::new(clean_url, model)))
        }
        _ => {
            anyhow::bail!("Provider '{:?}' is not yet supported in Rust core", config.provider);
        }
    }
}

// ヘルパトレイトでURLの末尾を判定
trait UrlHelper {
    fn ends_url_with_v1(&self) -> bool;
    fn ends_url_with_v1_slash(&self) -> bool;
}

impl UrlHelper for String {
    fn ends_url_with_v1(&self) -> bool {
        self.ends_with("/v1")
    }
    fn ends_url_with_v1_slash(&self) -> bool {
        self.ends_with("/v1/")
    }
}

// ==========================================
// 各プロバイダ クライアント実装
// ==========================================

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

    async fn unload(&self) -> Result<()> {
        // クラウドAPIのためVRAMアンロードは不要
        Ok(())
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
            keep_alive: "5m".to_string(),
        }
    }

    pub fn with_keep_alive(mut self, keep_alive: &str) -> Self {
        self.keep_alive = keep_alive.to_string();
        self
    }
}

#[async_trait]
impl LlmProvider for OllamaClient {
    async fn generate(&self, prompt: &str) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);

        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "keep_alive": self.keep_alive
        });

        let resp = self.client.post(&url).json(&body).send().await?;
        let json: serde_json::Value = resp.json().await?;

        let text = json["response"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to parse OllamaCenter response. Raw JSON: {:?}", json))?;

        Ok(text.to_string())
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/api/embeddings", self.base_url);

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

    async fn unload(&self) -> Result<()> {
        let url = format!("{}/api/generate", self.base_url);
        
        let body_main = serde_json::json!({
            "model": self.model,
            "prompt": "",
            "stream": false,
            "keep_alive": 0
        });
        let _ = self.client.post(&url).json(&body_main).send().await;

        let body_embed = serde_json::json!({
            "model": "nomic-embed-text",
            "prompt": "",
            "stream": false,
            "keep_alive": 0
        });
        let _ = self.client.post(&url).json(&body_embed).send().await;

        Ok(())
    }
}

