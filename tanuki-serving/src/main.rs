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
        .allow_methods([Method::GET, Method::POST]);

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/propose", post(handle_proposal))
        .route("/api/nodes", get(get_nodes))
        .route("/api/clusters", get(get_clusters))
        .route("/api/search", get(search_nodes))
        .route("/api/vector-search", post(vector_search))
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
    let q = params.q.unwrap_or_default().to_lowercase();
    use unicode_normalization::UnicodeNormalization;
    let normalized_q = q.nfkc().collect::<String>();
    let keywords: Vec<&str> = normalized_q.split_whitespace().collect();

    println!("🔍 Searching for: {:?} (original: {:?})", keywords, q);

    let db = state.db.lock().unwrap();
    let nodes = db.get_all_nodes().unwrap_or_default();

    let filtered: Vec<_> = nodes.into_iter().filter(|n| {
        let title_norm = n.title.to_lowercase().as_str().nfkc().collect::<String>();
        let content_norm = n.content.to_lowercase().as_str().nfkc().collect::<String>();
        
        // 全キーワードが含まれているかチェック
        keywords.iter().all(|&kw| {
            title_norm.contains(kw) || content_norm.contains(kw)
        })
    }).collect();

    println!("  ✅ Found {} results", filtered.len());
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
