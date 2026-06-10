use anyhow::Result;
use tanuki_compiler::{
    parse_markdown_file, process_nodes, reduce_knowledge, 
    generate_tree, generate_checkpoint, calculate_ast_root_hash
};
use tanuki_core::llm::OllamaClient;
use tanuki_core::db::TanukiDb;
use tanuki_core::Checkpoint;
use walkdir::WalkDir;
use std::path::Path;
use std::time::UNIX_EPOCH;
use std::collections::HashSet;
use std::fs;
use uuid::Uuid;

const CHECKPOINT_PATH: &str = ".gemini/memory/tanuki_checkpoint.json";

#[tokio::main]
async fn main() -> Result<()> {
    // 軽量ビルドモード: TANUKI_NO_REDUCE=1 を設定するとReduce処理をスキップする
    let no_reduce = std::env::var("TANUKI_NO_REDUCE").map(|v| v == "1").unwrap_or(false);
    if no_reduce {
        println!("T.A.N.U.K.I. Compiler starting... [FAST MODE: Reduce phase skipped]");
    } else {
        println!("T.A.N.U.K.I. Compiler starting...");
    }

    let center_url = "http://localhost:11434".to_string();
    let model_name = std::env::var("TANUKI_MODEL").unwrap_or_else(|_| "gemma4:e4b".to_string());
    println!("  Unified Model Selected: {}", model_name);
    
    let unified_llm = OllamaClient::new(center_url, model_name);

    let res = run_pipeline(no_reduce, &unified_llm).await;

    // VRAMの防衛: 成功・失敗を問わず、必ずOllamaのVRAMモデルをアンロードする
    println!("  🧹 VRAM Guard: Unloading model from GPU...");
    let _ = unified_llm.unload().await;

    res
}

async fn run_pipeline(no_reduce: bool, unified_llm: &OllamaClient) -> Result<()> {
    // 対象ディレクトリのリスト (プロジェクトルートからの相対パス)
    let target_dirs = vec![
        "../../Documents/InBox",
        "../../Documents/Archive/Devlog",
        "../../Documents/Archive/Media",
        "../../Documents/Archive/Specifications",
    ];

    // Phase -1: Load Checkpoint (Resume Protocol)
    let current_hash = calculate_ast_root_hash(&target_dirs)?;
    let mut session_id = Uuid::new_v4().to_string();
    let mut strategist_notes = String::new();

    if Path::new(CHECKPOINT_PATH).exists() {
        let content = fs::read_to_string(CHECKPOINT_PATH)?;
        if let Ok(checkpoint) = serde_json::from_str::<Checkpoint>(&content) {
            if checkpoint.ast_root_hash == current_hash {
                println!("  🔄 Resume Protocol: AST Hash matched!");
                println!("  📜 Strategist Notes: \"{}\"", checkpoint.strategist_notes);
                session_id = checkpoint.session_id; // セッションIDを継続
                strategist_notes = checkpoint.strategist_notes;
                
                // 完全一致の場合、何もせず終了する選択肢もあるが、
                // 今回は「起動高速化」として、以降の増分チェックは一瞬で終わるはず。
            } else {
                println!("  ⚠️ Resume Protocol: AST Hash mismatch. Code base updated.");
            }
        }
    }
    
    let mut db = TanukiDb::open("knowledge.db")?;
    
    let mut processed_files = HashSet::new();
    let mut any_changed = false;

    for dir in &target_dirs {
        println!("  Scanning directory: {}", dir);
        if !Path::new(dir).exists() {
            println!("  Warning: Directory {} does not exist. Skipping.", dir);
            continue;
        }

        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                let path_str = path.to_string_lossy().to_string();
                processed_files.insert(path_str.clone());
                
                let metadata = entry.metadata()?;
                let mtime = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();
                
                // DBチェック
                let stored_mtime = db.get_file_mtime(&path_str)?;
                
                if let Some(stored) = stored_mtime {
                    if stored == mtime {
                        // ハッシュが一致している場合はスキップ（ログは出さないか、デバッグのみ）
                        continue;
                    }
                }

                println!("    Processing: {:?}", path);
                
                // 古いノードを削除（IDが変わる可能性があるため）
                db.delete_nodes_by_source(&path_str)?;
                
                match parse_markdown_file(path) {
                    Ok(nodes) => {
                        if !nodes.is_empty() {
                            println!("    Processing {} nodes...", nodes.len());
                            process_nodes(nodes, unified_llm, &mut db).await?;
                            any_changed = true;
                        }
                        db.upsert_file_meta(&path_str, mtime)?;
                    }
                    Err(e) => {
                        println!("    Error parsing {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    // Phase 1.5: Cleanup (削除されたファイルのメタ情報とノードを削除)
    let all_stored_meta = db.get_all_file_meta()?;
    for meta in all_stored_meta {
        if !processed_files.contains(&meta.path) {
            println!("  Cleanup: Removing deleted file data: {}", meta.path);
            db.delete_nodes_by_source(&meta.path)?;
            db.delete_file_meta(&meta.path)?;
        }
    }

    if any_changed {
        if no_reduce {
            println!("  Frontend: Processing complete. [FAST MODE] Skipping Reduce phase.");
        } else {
            println!("  Frontend: Processing complete. Starting Reduction...");
            // Phase 3: Middle-end (Reduce)
            reduce_knowledge(unified_llm, &mut db).await?;
            println!("  Middle-end: Reduction complete.");
        }
    } else {
        println!("  No changes detected. Knowledge base is up to date.");
    }
    
    // Phase 4: Backend (Generate Tree)
    // 常に最新のロジックでツリーを再生成する
    generate_tree(&db, "output_knowledge")?;
    println!("  Backend: Tree generation complete.");
    
    // Phase 5: Packing (Export Checkpoint)
    println!("  Phase 5: Packing AST Knowledge Tree...");
    let checkpoint = generate_checkpoint(&db, &target_dirs, &session_id)?;
    
    let checkpoint_dir = Path::new(CHECKPOINT_PATH).parent().unwrap();
    if !checkpoint_dir.exists() {
        fs::create_dir_all(checkpoint_dir)?;
    }
    
    let json = serde_json::to_string_pretty(&checkpoint)?;
    fs::write(CHECKPOINT_PATH, json)?;
    println!("  ✓ Checkpoint saved to: {}", CHECKPOINT_PATH);

    println!("Pipeline finished successfully!");
    Ok(())
}
