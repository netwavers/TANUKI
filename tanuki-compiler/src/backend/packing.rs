use tanuki_core::{TanukiDb, Checkpoint, SummaryNode, KnowledgeType};
use anyhow::Result;
use sha2::{Sha256, Digest};
use std::path::Path;
use walkdir::WalkDir;
use std::fs;

pub fn calculate_ast_root_hash(target_dirs: &[&str]) -> Result<String> {
    let mut hasher = Sha256::new();
    
    // ソースファイルのリストを決定論的にソートしてハッシュ化
    let mut files = Vec::new();
    for dir in target_dirs {
        // 相対パスを解決（実行ディレクトリからのパス）
        if !Path::new(dir).exists() {
            println!("      Warning: Directory {} not found for hashing.", dir);
            continue;
        }
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext == "md") {
                files.push(entry.path().to_path_buf());
            }
        }
    }
    files.sort();

    for file_path in files {
        let content = fs::read(&file_path)?;
        // パスと内容の両方をハッシュに含める
        hasher.update(file_path.to_string_lossy().as_bytes());
        hasher.update(&content);
    }

    Ok(hex::encode(hasher.finalize()))
}

pub fn generate_checkpoint(db: &TanukiDb, target_dirs: &[&str], session_id: &str) -> Result<Checkpoint> {
    let hash = calculate_ast_root_hash(target_dirs)?;
    let mut checkpoint = Checkpoint::new(session_id.to_string(), hash);

    // DBからノードサマリーを取得
    let nodes = db.get_all_nodes()?;
    checkpoint.knowledge_graph_summary.nodes = nodes.into_iter().map(|n| SummaryNode {
        id: n.id,
        node_type: KnowledgeType::Node, // 今後メタデータから動的に判定可能
        symbol: n.title,
        link: format!("file://{}", n.source_path),
    }).collect();

    // ホットゾーン（対象ディレクトリ）の記録
    checkpoint.knowledge_graph_summary.hot_zones = target_dirs.iter().map(|s| s.to_string()).collect();

    checkpoint.strategist_notes = format!(
        "前回セッション完了: {} 個のノードをパッキング済み。ASTハッシュ: {}",
        checkpoint.knowledge_graph_summary.nodes.len(),
        checkpoint.ast_root_hash
    );

    Ok(checkpoint)
}
