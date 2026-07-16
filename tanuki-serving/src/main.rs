// Copyright (c) 2026 かぜまる (Kazemaru) / Antigravity AI.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// ---
// 🐾 T.A.N.U.K.I. Project - Tactical Agentic Network Core Serving Engine
// "世界の木（イルミンシュール）の秩序は、このライセンスによって厳格に守護されます。"

use anyhow::Result;
use axum::{
    routing::{get, post},
    Json, Router, extract::{State, Query},
    http::{Method},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::path::Path;
use tower_http::cors::CorsLayer;
use tanuki_core::db::{TanukiDb, KnowledgeNode, Cluster};
use tanuki_core::MmapMemoryManager;
use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize, Serialize)]
pub struct ProposalRequest {
    pub source_id: String,
    pub target_id: String,
    pub link_type: String,
    pub strength: f32,
    pub rationale: String,
}

#[derive(Debug, Serialize)]
pub struct ProposalResponse {
    pub score: f32,
    pub connectivity: f32,
    pub recommendation: String,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VectorSearchRequest {
    pub vector: Vec<f32>,
    pub top_k: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct VectorSearchResponse {
    pub node_id: u64,
    pub score: f32,
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    pub limit: Option<usize>,
}

struct AppState {
    db: Mutex<TanukiDb>,
    mmap_memory: Option<MmapMemoryManager>,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🐾 T.A.N.U.K.I. Serving API starting...");

    let db = TanukiDb::open("knowledge.db")?;
    
    // MmapMemoryManagerの初期化（バイナリファイルが存在する場合）
    let mmap_path = "knowledge.bin";
    let mmap_memory = if Path::new(mmap_path).exists() {
        println!("  🧠 Mapping memory file: {}", mmap_path);
        Some(MmapMemoryManager::new(mmap_path)?)
    } else {
        println!("  ⚠️ Memory binary file not found. Mmap search will be unavailable.");
        None
    };

    let shared_state = Arc::new(AppState { 
        db: Mutex::new(db),
        mmap_memory,
    });

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(tower_http::cors::Any);

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/propose", post(handle_proposal))
        .route("/api/nodes", get(get_nodes))
        .route("/api/clusters", get(get_clusters))
        .route("/api/search", get(search_nodes))
        .route("/api/vector-search", post(vector_search))
        .route("/api/rag-summary", get(get_rag_summary))
        .route("/api/system-status", get(get_system_status))
        .route("/api/playground/prune", post(handle_playground_prune))
        .route("/api/playground/infer", post(handle_playground_infer))
        .layer(cors)
        .with_state(shared_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("  🚀 Serving at http://{}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

fn is_empty_or_title_only(title: &str, content: &str) -> bool {
    let trimmed_content = content.trim();
    if trimmed_content.is_empty() {
        return true;
    }
    let clean_title: String = title.chars().filter(|c| !c.is_whitespace() && *c != '#').collect();
    let clean_content: String = trimmed_content.chars().filter(|c| !c.is_whitespace() && *c != '#').collect();
    clean_title == clean_content || clean_content == "AISummaryPlaceholder"
}

fn clean_query_to_keywords(q: &str) -> Vec<String> {
    let normalized = {
        use unicode_normalization::UnicodeNormalization;
        q.to_lowercase().nfkc().collect::<String>()
    };
    
    let mut words = Vec::new();
    let mut current_word = String::new();
    let mut last_is_alnum = None;
    
    for c in normalized.chars() {
        if c.is_whitespace() {
            if !current_word.is_empty() {
                words.push(current_word.clone());
                current_word.clear();
            }
            last_is_alnum = None;
            continue;
        }
        
        let is_alnum = c.is_alphanumeric() && c.is_ascii();
        if let Some(prev) = last_is_alnum {
            if prev != is_alnum {
                if !current_word.is_empty() {
                    words.push(current_word.clone());
                    current_word.clear();
                }
            }
        }
        current_word.push(c);
        last_is_alnum = Some(is_alnum);
    }
    
    if !current_word.is_empty() {
        words.push(current_word);
    }
    
    let stopwords = ["about", "について", "とは", "の", "は", "が", "を", "に", "へ", "と", "で", "から", "より", "まで"];
    words.into_iter()
        .map(|w| w.trim().to_string())
        .filter(|w| !w.is_empty() && !stopwords.contains(&w.as_str()))
        .collect()
}

async fn root() -> &'static str {
    "🐾 T.A.N.U.K.I. Serving API is running! Try /health for more info."
}

async fn health_check() -> &'static str {
    "TANUKI Serving is online! 🐾"
}

async fn get_nodes(State(state): State<Arc<AppState>>) -> Json<Vec<KnowledgeNode>> {
    let db = state.db.lock().unwrap();
    let nodes = db.get_all_nodes().unwrap_or_default();
    Json(nodes)
}

async fn get_clusters(State(state): State<Arc<AppState>>) -> Json<Vec<Cluster>> {
    let db = state.db.lock().unwrap();
    let clusters = db.get_all_clusters().unwrap_or_default();
    Json(clusters)
}

async fn search_nodes(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let q = params.q.unwrap_or_default();
    if q.trim().is_empty() {
        return Json(vec![]);
    }
    let limit = params.limit.unwrap_or(10);

    let use_vector_search = state.mmap_memory.is_some();

    if use_vector_search {
        let model_name = std::env::var("TANUKI_MODEL").unwrap_or_else(|_| "gemma4:e2b".to_string());
        let config_path = "config/models_config.json";

        println!("🔍 Semantic searching for: {:?}", q);

        let query_vector = match tanuki_core::load_provider(config_path, &model_name) {
            Ok(provider) => {
                match provider.embed(&q).await {
                    Ok(vec) => {
                        let _ = provider.unload().await;
                        Some(vec)
                    }
                    Err(e) => {
                        let _ = provider.unload().await;
                        println!("  ⚠️ Embedding generation failed: {}. Falling back.", e);
                        None
                    }
                }
            }
            Err(e) => {
                println!("  ⚠️ Failed to load provider: {}. Falling back.", e);
                None
            }
        };

        if let Some(vector) = query_vector {
            if vector.len() == 768 {
                let mut query_array = [0.0f32; 768];
                query_array.copy_from_slice(&vector[0..768]);

                let mmap = state.mmap_memory.as_ref().unwrap();
                match mmap.search(&query_array, limit * 2) {
                    Ok(results) => {
                        // しきい値（環境変数 TANUKI_SEMANTIC_THRESHOLD、既定 0.35）
                        let threshold: f32 = std::env::var("TANUKI_SEMANTIC_THRESHOLD")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0.35);
                        let filtered_results: Vec<_> = results.into_iter()
                            .filter(|&(_, score)| score >= threshold)
                            .collect();

                        let db = state.db.lock().unwrap();
                        let all_nodes = db.get_all_nodes().unwrap_or_default();

                        let mut matched_nodes = Vec::new();
                        for (f_id, _score) in filtered_results {
                            if let Some(node) = all_nodes.iter().find(|n| calculate_fnv1a(&n.context_path) == f_id) {
                                // 本文が完全に空のノードのみ除外（タイトル同一でも検索リストには表示させる）
                                if !node.content.trim().is_empty() {
                                    matched_nodes.push(node.clone());
                                }
                            }
                        }
                        matched_nodes.truncate(limit);
                        println!("  ✅ Found {} semantic results", matched_nodes.len());
                        return Json(matched_nodes);
                    }
                    Err(e) => {
                        println!("  ⚠️ Vector search failed: {}. Falling back.", e);
                    }
                }
            }
        }
    }

    // フォールバック: キーワード部分一致検索
    println!("🔍 Keyword AND searching for: {:?}", q);
    let keywords = clean_query_to_keywords(&q);

    let db = state.db.lock().unwrap();
    let nodes = db.get_all_nodes().unwrap_or_default();

    let filtered: Vec<_> = nodes.into_iter().filter(|n| {
        // 本文が完全に空のノードのみ除外
        if n.content.trim().is_empty() {
            return false;
        }
        use unicode_normalization::UnicodeNormalization;
        let title_norm = n.title.to_lowercase().as_str().nfkc().collect::<String>();
        let content_norm = n.content.to_lowercase().as_str().nfkc().collect::<String>();
        
        keywords.iter().all(|kw| {
            title_norm.contains(kw) || content_norm.contains(kw)
        })
    }).collect();

    println!("  ✅ Found {} keyword results", filtered.len());
    Json(filtered)
}

async fn vector_search(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<VectorSearchRequest>,
) -> Json<Vec<VectorSearchResponse>> {
    if let Some(ref mmap) = state.mmap_memory {
        let top_k = payload.top_k.unwrap_or(5);
        
        if payload.vector.len() != 768 {
            println!("  ⚠️ Vector search requested with invalid dimension: {}", payload.vector.len());
            return Json(vec![]);
        }
        
        let mut query_array = [0.0f32; 768];
        query_array.copy_from_slice(&payload.vector[0..768]);

        match mmap.search(&query_array, top_k) {
            Ok(results) => {
                let response = results.into_iter()
                    .map(|(node_id, score)| VectorSearchResponse { node_id, score })
                    .collect();
                Json(response)
            }
            Err(e) => {
                println!("  ❌ Vector search error: {}", e);
                Json(vec![])
            }
        }
    } else {
        println!("  ⚠️ Vector search requested but mmap_memory is not initialized.");
        Json(vec![])
    }
}

async fn handle_proposal(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ProposalRequest>,
) -> Json<ProposalResponse> {
    // Existing logic...
    println!("🐾 Received proposal: {} -> {} ({})", payload.source_id, payload.target_id, payload.link_type);
    let db = state.db.lock().map_err(|_| "Lock failed").unwrap();
    let mut tx = db.start_transaction();
    
    match tx.insert_link_speculative(&payload.source_id, &payload.target_id, &payload.link_type, payload.strength) {
        Ok(_) => {
            let evaluator = tanuki_compiler::middle::evaluator::SpeculativeEvaluator::new(&db);
            match evaluator.evaluate_proposal() {
                Ok(result) => {
                    let (status, _) = if result.score > 0.6 {
                        tx.commit();
                        ("Accepted".to_string(), true)
                    } else {
                        let _ = tx.rollback();
                        ("Rejected".to_string(), false)
                    };
                    Json(ProposalResponse {
                        score: result.score,
                        connectivity: result.connectivity,
                        recommendation: result.recommendation,
                        status,
                    })
                }
                Err(_) => {
                    let _ = tx.rollback();
                    Json(ProposalResponse { score: 0.0, connectivity: 0.0, recommendation: "Eval failed".into(), status: "Error".into() })
                }
            }
        }
        Err(_) => Json(ProposalResponse { score: 0.0, connectivity: 0.0, recommendation: "Op failed".into(), status: "Error".into() })
    }
}

#[derive(Debug, Serialize)]
pub struct RagSummaryResponse {
    pub answer: String,
    pub references: Vec<String>,
}

async fn get_rag_summary(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let q = params.q.unwrap_or_default();
    let limit_param = params.limit.unwrap_or(3); // デフォルト3件

    if q.trim().is_empty() {
        return Json(RagSummaryResponse {
            answer: "質問を入力してくださいわ！🐾".to_string(),
            references: vec![],
        });
    }

    let mut top_nodes = Vec::new();
    let use_vector_search = state.mmap_memory.is_some();

    if use_vector_search {
        let model_name = std::env::var("TANUKI_MODEL").unwrap_or_else(|_| "gemma4:e2b".to_string());
        let config_path = "config/models_config.json";

        let query_vector = match tanuki_core::load_provider(config_path, &model_name) {
            Ok(provider) => {
                match provider.embed(&q).await {
                    Ok(vec) => {
                        let _ = provider.unload().await;
                        Some(vec)
                    }
                    Err(e) => {
                        let _ = provider.unload().await;
                        println!("  ⚠️ RAG Embedding generation failed: {}. Falling back.", e);
                        None
                    }
                }
            }
            Err(e) => {
                println!("  ⚠️ RAG Failed to load provider: {}. Falling back.", e);
                None
            }
        };

        if let Some(vector) = query_vector {
            if vector.len() == 768 {
                let mut query_array = [0.0f32; 768];
                query_array.copy_from_slice(&vector[0..768]);

                let mmap = state.mmap_memory.as_ref().unwrap();
                if let Ok(results) = mmap.search(&query_array, limit_param * 2) {
                    if !results.is_empty() {
                        let max_score = results[0].1;
                        println!("  ℹ️ RAG Semantic search max score: {:.4}", max_score);
                        if max_score < 0.75 {
                            println!("  🛡️ Hallucination Guard: Max score {:.4} is below threshold 0.75. Skipping LLM generation.", max_score);
                            return Json(RagSummaryResponse {
                                answer: "申し訳ありませんわ、提供されたナレッジベースからはその質問に関する情報が見つかりませんでした。".to_string(),
                                references: vec![],
                            });
                        }
                    }

                    let filtered_results: Vec<_> = results.into_iter()
                        .filter(|&(_, score)| score >= 0.40)
                        .collect();

                    let db = state.db.lock().unwrap();
                    let all_nodes = db.get_all_nodes().unwrap_or_default();

                    for (f_id, _score) in filtered_results {
                        if let Some(node) = all_nodes.iter().find(|n| calculate_fnv1a(&n.context_path) == f_id) {
                            // 本文が空、または見出し（タイトル）のみのノードは、LLM要約用のRAGコンテキストから事前に除外する
                            if !is_empty_or_title_only(&node.title, &node.content) {
                                top_nodes.push(node.clone());
                            }
                        }
                    }
                    top_nodes.truncate(limit_param);
                }
            }
        }
    }

    if top_nodes.is_empty() {
        let keywords = clean_query_to_keywords(&q);

        let db = state.db.lock().unwrap();
        let nodes = db.get_all_nodes().unwrap_or_default();

        let mut filtered: Vec<_> = nodes.into_iter().filter(|n| {
            // 本文が空、または見出し（タイトル）のみのノードは、LLM要約用のRAGコンテキストから事前に除外する
            if is_empty_or_title_only(&n.title, &n.content) {
                return false;
            }
            use unicode_normalization::UnicodeNormalization;
            let title_norm = n.title.to_lowercase().as_str().nfkc().collect::<String>();
            let content_norm = n.content.to_lowercase().as_str().nfkc().collect::<String>();
            
            keywords.iter().all(|kw| {
                title_norm.contains(kw) || content_norm.contains(kw)
            })
        }).collect();
        
        filtered.truncate(limit_param);
        top_nodes = filtered;
    }

    let limit = top_nodes.len();
    if limit == 0 {
        return Json(RagSummaryResponse {
            answer: "該当するナレッジが見つかりませんでしたわ。別のキーワードでお試しください。".to_string(),
            references: vec![],
        });
    }

    // 返却用およびLLM用のコンテキスト構築
    let ref_titles: Vec<String> = top_nodes.iter().map(|n| n.title.clone()).collect();
    let mut context_str = String::new();
    let use_summary = limit > 3;

    for (i, node) in top_nodes.iter().enumerate() {
        let info = if use_summary {
            node.summary.clone()
        } else {
            node.content.clone()
        };
        context_str.push_str(&format!(
            "【参考資料 {}】\nタイトル: {}\n{}:\n{}\n\n",
            i + 1, node.title, if use_summary { "概要" } else { "内容" }, info
        ));
    }

    let prompt = format!(
        "あなたはご主人様に仕えるメイド風アシスタントの「たぬきちゃん」です。丁寧で愛嬌のあるお嬢様メイド口調（〜ですわ、〜ですの、等）で、提供された【参考資料】に記載されている事実に基づいて、質問「{}」に親身に回答してください。\n\n\
        --- 参考資料 ---\n\
        {}",
        q, context_str
    );

    let model_name = std::env::var("TANUKI_MODEL").unwrap_or_else(|_| "gemma4:e2b".to_string());
    let config_path = "config/models_config.json";

    println!("🤖 RAG Generating answer using model: {} (Context: {} nodes, Mode: {})", model_name, limit, if use_summary { "FullSummary" } else { "Quick" });

    let answer = match tanuki_core::load_provider(config_path, &model_name) {
        Ok(provider) => {
            match provider.generate(&prompt).await {
                Ok(generated) => {
                    let _ = provider.unload().await;
                    generated
                }
                Err(e) => {
                    let _ = provider.unload().await;
                    format!("エラー：LLMでの回答生成に失敗しました（{}）", e)
                }
            }
        }
        Err(e) => {
            format!("エラー：LLMプロバイダのロードに失敗しました（{}）。config/models_config.json が存在するか確認してください。", e)
        }
    };

    Json(RagSummaryResponse {
        answer,
        references: ref_titles,
    })
}

fn calculate_fnv1a(s: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for &byte in s.as_bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3u64);
    }
    hash
}

#[derive(Debug, Serialize)]
pub struct SystemStatusResponse {
    pub database_size_bytes: u64,
    pub database_modified_time: String,
    pub mmap_size_bytes: u64,
    pub mmap_modified_time: String,
    pub total_nodes: usize,
    pub total_clusters: usize,
    pub ollama_online: bool,
    pub ollama_model: String,
    pub vram_total_mb: Option<u32>,
    pub vram_used_mb: Option<u32>,
    pub vram_free_mb: Option<u32>,
}

async fn get_system_status(State(state): State<Arc<AppState>>) -> Json<SystemStatusResponse> {
    // 1. データベース情報
    let db_path = "knowledge.db";
    let (db_size, db_mtime) = get_file_metadata(db_path);

    let mmap_path = "knowledge.bin";
    let (mmap_size, mmap_mtime) = get_file_metadata(mmap_path);

    let (total_nodes, total_clusters) = {
        let db = state.db.lock().unwrap();
        let nodes = db.get_all_nodes().unwrap_or_default().len();
        let clusters = db.get_all_clusters().unwrap_or_default().len();
        (nodes, clusters)
    };

    // 2. Ollama 状態
    let ollama_base_url = std::env::var("OLLAMA_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    let ollama_model = std::env::var("TANUKI_MODEL")
        .or_else(|_| std::env::var("OLLAMA_MODEL"))
        .unwrap_or_else(|_| "gemma4:e2b".to_string());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap();

    let ollama_online = match client.get(format!("{}/api/tags", ollama_base_url)).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    };

    // 3. VRAM 状態 (nvidia-smi 実行)
    let (vram_total, vram_used, vram_free) = get_gpu_vram_info();

    Json(SystemStatusResponse {
        database_size_bytes: db_size,
        database_modified_time: db_mtime,
        mmap_size_bytes: mmap_size,
        mmap_modified_time: mmap_mtime,
        total_nodes,
        total_clusters,
        ollama_online,
        ollama_model,
        vram_total_mb: vram_total,
        vram_used_mb: vram_used,
        vram_free_mb: vram_free,
    })
}

fn get_file_metadata(path: &str) -> (u64, String) {
    if let Ok(meta) = std::fs::metadata(path) {
        let size = meta.len();
        let mtime_str = if let Ok(modified) = meta.modified() {
            let dt: DateTime<Utc> = modified.into();
            dt.to_rfc3339()
        } else {
            "Unknown".to_string()
        };
        (size, mtime_str)
    } else {
        (0, "Not Found".to_string())
    }
}

fn get_gpu_vram_info() -> (Option<u32>, Option<u32>, Option<u32>) {
    let output = if cfg!(target_os = "windows") || cfg!(target_os = "linux") {
        std::process::Command::new("nvidia-smi")
            .args(&["--query-gpu=memory.total,memory.used,memory.free", "--format=csv,noheader,nounits"])
            .output()
    } else {
        return (None, None, None);
    };

    match output {
        Ok(out) if out.status.success() => {
            let stdout_str = String::from_utf8_lossy(&out.stdout);
            let parts: Vec<&str> = stdout_str.trim().split(',').map(|s| s.trim()).collect();
            if parts.len() == 3 {
                let total = parts[0].parse::<u32>().ok();
                let used = parts[1].parse::<u32>().ok();
                let free = parts[2].parse::<u32>().ok();
                (total, used, free)
            } else {
                (None, None, None)
            }
        }
        _ => (None, None, None),
    }
}

// ==========================================
// Playground (Core Visualizer) APIs
// ==========================================

#[derive(Debug, Deserialize)]
pub struct PlaygroundNodeInput {
    pub title: String,
    pub body: String,
    pub tier: u8,
}

#[derive(Debug, Deserialize)]
pub struct PlaygroundPruneRequest {
    pub system_constraint: String,
    pub task_instruction: String,
    pub target_limit: u32,
    pub nodes: Vec<PlaygroundNodeInput>,
}

#[derive(Debug, Serialize)]
pub struct PlaygroundNodeOutput {
    pub node_id: u64,
    pub title: String,
    pub body: String,
    pub tier: u8,
    pub tokens: u32,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct PlaygroundPruneResponse {
    pub initial_tokens: u32,
    pub pruned_tokens: u32,
    pub final_tokens: u32,
    pub dsl_output: String,
    pub nodes: Vec<PlaygroundNodeOutput>,
}

async fn handle_playground_prune(
    Json(payload): Json<PlaygroundPruneRequest>,
) -> Json<PlaygroundPruneResponse> {
    use tanuki_core::{FlatAST};

    let mut ast = FlatAST::new();

    // 1. システム制約 (System, node_type = 0) を push
    let sys_id = calculate_fnv1a(&payload.system_constraint);
    ast.push_node(
        sys_id,
        0, // System
        0, // priority = 0 (critical)
        false, // is_subnode
        0, // child_count
        &payload.system_constraint,
    );

    // 2. タスク指示 (Instruction, node_type = 1) を push
    let inst_id = calculate_fnv1a(&payload.task_instruction);
    ast.push_node(
        inst_id,
        1, // Instruction
        0, // priority = 0 (critical)
        false, // is_subnode
        0, // child_count
        &payload.task_instruction,
    );

    // 3. 親ノードを push (Reference, node_type = 2, priority = 0, is_subnode = false)
    let parent_id = calculate_fnv1a("PlaygroundDocument");
    ast.push_node(
        parent_id,
        2, // Reference
        0, // priority = 0
        false, // is_subnode
        payload.nodes.len() as u16, // child_count
        "Playground Document Context",
    );

    // 4. 各ノードをサブノードとして追加
    for node in &payload.nodes {
        let node_payload = format!("{}\n{}", node.title, node.body);
        let node_id = calculate_fnv1a(&node_payload);
        ast.push_node(
            node_id,
            2, // Reference
            node.tier, // priority = tier
            true, // is_subnode = true
            0, // child_count
            &node_payload,
        );
    }

    let initial_tokens = ast.total_tokens();
    ast.prune(payload.target_limit);
    let final_tokens = ast.total_tokens();
    let pruned_tokens = initial_tokens.saturating_sub(final_tokens);
    let dsl_output = ast.render_dsl();

    // 5. 削減結果をパースして返す
    let mut out_nodes = Vec::new();
    for (header, payload_str, _) in ast.iter_nodes() {
        if header.is_subnode() {
            let parts: Vec<&str> = payload_str.splitn(2, '\n').collect();
            let title = parts.get(0).unwrap_or(&"").to_string();
            let body = parts.get(1).unwrap_or(&"").to_string();

            out_nodes.push(PlaygroundNodeOutput {
                node_id: header.node_id,
                title,
                body,
                tier: header.priority,
                tokens: header.payload_len,
                is_active: header.is_active(),
            });
        }
    }

    Json(PlaygroundPruneResponse {
        initial_tokens,
        pruned_tokens,
        final_tokens,
        dsl_output,
        nodes: out_nodes,
    })
}

#[derive(Debug, Deserialize)]
pub struct PlaygroundInferRequest {
    pub dsl_output: String,
    pub model_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PlaygroundInferResponse {
    pub response: String,
}

async fn handle_playground_infer(
    Json(payload): Json<PlaygroundInferRequest>,
) -> impl axum::response::IntoResponse {
    let model_name = payload.model_name
        .unwrap_or_else(|| std::env::var("TANUKI_MODEL").unwrap_or_else(|_| "gemma4:e2b".to_string()));
    let config_path = "config/models_config.json";

    println!("🤖 Playground Infer executing model: {}", model_name);

    // VRAM排他制御ロックを獲得！
    let _vram_guard = tanuki_core::GLOBAL_VRAM_LOCK.lock().await;

    // DSL を自然言語プロンプトにデシリアライズ！
    let formatted_prompt = deserialize_dsl_to_prompt(&payload.dsl_output);
    println!("🔥 [DEBUG] Formatted Prompt sent to LLM:\n{}\n====================", formatted_prompt);

    let result = match tanuki_core::load_provider(config_path, &model_name) {
        Ok(provider) => {
            match provider.generate(&formatted_prompt).await {
                Ok(generated) => {
                    let _ = provider.unload().await;
                    Ok(generated)
                }
                Err(e) => {
                    let _ = provider.unload().await;
                    Err(format!("LLM推論エラー: {}", e))
                }
            }
        }
        Err(e) => {
            Err(format!("プロバイダロードエラー: {}", e))
        }
    };

    match result {
        Ok(res) => (axum::http::StatusCode::OK, Json(PlaygroundInferResponse { response: res })).into_response(),
        Err(err) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
    }
}

/// 超軽量 DSL（#S:, #I:, #R[...]）をパースし、LLM が正しく指示に従えるように
/// チャット形式のプレーンな自然言語プロンプトにデシリアライズ（復元・整形）します。
fn deserialize_dsl_to_prompt(dsl: &str) -> String {
    use regex::Regex;

    let mut system_instructions = Vec::new();
    let mut task_instructions = Vec::new();
    let mut references = Vec::new();
    
    let mut current_ref_title = String::new();
    let mut current_ref_body = Vec::new();

    // サブノードを処理するための正規表現
    let subnode_regex = Regex::new(r"^\s*└─\s*#Sub\[\d+\]:\s*(.+)$").unwrap();

    for line in dsl.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("#S:") {
            system_instructions.push(trimmed["#S:".len()..].trim().to_string());
        } else if trimmed.starts_with("#I:") {
            task_instructions.push(trimmed["#I:".len()..].trim().to_string());
        } else if trimmed.starts_with("#R[") {
            // 前の参照ノードを保存
            if !current_ref_title.is_empty() {
                references.push(format!("### {}\n{}", current_ref_title, current_ref_body.join("\n")));
                current_ref_body.clear();
            }
            // 新しい参照ノードのパース
            if let Some(pos) = trimmed.find("]:") {
                let title = trimmed[pos + 2..].trim().to_string();
                current_ref_title = title;
            }
        } else if let Some(caps) = subnode_regex.captures(line) {
            // サブノードの本文を追加
            let payload = caps.get(1).unwrap().as_str().trim();
            current_ref_body.push(payload.to_string());
        } else {
            // タグのないプレーンテキスト等
            if !current_ref_title.is_empty() {
                current_ref_body.push(trimmed.to_string());
            }
        }
    }

    // 最後の参照ノードを保存
    if !current_ref_title.is_empty() {
        references.push(format!("### {}\n{}", current_ref_title, current_ref_body.join("\n")));
    }

    // 自然言語プロンプトの組み立て
    // ローカル LLM（gemma4など）が「以下の文書」「上記の文書」という位置依存の指示語を正しく解釈できるように、
    // 【システム指示】➔【タスク指示】➔【参照ドキュメント】の順番で結合します。
    let mut prompt = String::new();

    if !system_instructions.is_empty() {
        prompt.push_str("【システム指示】\n");
        for sys in system_instructions {
            prompt.push_str(&format!("{}\n", sys));
        }
        prompt.push_str("\n");
    }

    if !task_instructions.is_empty() {
        prompt.push_str("【タスク指示】\n");
        prompt.push_str("与えられた【参照ドキュメント】の内容のみに基づき、以下の命令に従って回答してください：\n");
        for inst in task_instructions {
            prompt.push_str(&format!("- {}\n", inst));
        }
        prompt.push_str("\n");
    }

    if !references.is_empty() {
        prompt.push_str("【参照ドキュメント】\n");
        for re in references {
            prompt.push_str(&format!("{}\n\n", re));
        }
    }

    prompt
}
