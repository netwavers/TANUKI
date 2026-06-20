use anyhow::Result;
use tanuki_compiler::{
    parse_markdown_file, process_nodes, reduce_knowledge, 
    generate_tree, generate_checkpoint, calculate_ast_root_hash
};
use tanuki_core::llm::{LlmProvider, load_provider};
use tanuki_core::db::TanukiDb;
use tanuki_core::Checkpoint;
use walkdir::WalkDir;
use std::path::Path;
use std::time::UNIX_EPOCH;
use std::collections::HashSet;
use std::fs;
use uuid::Uuid;
use unicode_normalization::UnicodeNormalization;

const CHECKPOINT_PATH: &str = ".gemini/memory/tanuki_checkpoint.json";

enum SubCommand {
    Compile {
        db_path: String,
        src_dirs: Option<Vec<String>>,
        config_path: String,
        no_reduce: bool,
    },
    Query {
        db_path: String,
        query_string: String,
    },
    Help,
}

struct CliArgs {
    cmd: SubCommand,
}

fn parse_args() -> Result<CliArgs> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return Ok(CliArgs { cmd: SubCommand::Help });
    }

    let subcmd = &args[1];
    match subcmd.as_str() {
        "compile" => {
            let mut db_path = "knowledge.db".to_string();
            let mut src_dirs = None;
            let mut config_path = "config/models_config.json".to_string();
            let mut no_reduce = false;

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--db" => {
                        if i + 1 < args.len() {
                            db_path = args[i + 1].clone();
                            i += 2;
                        } else {
                            anyhow::bail!("Missing value for --db");
                        }
                    }
                    "--src" => {
                        if i + 1 < args.len() {
                            let dirs = args[i + 1].split(',').map(|s| s.trim().to_string()).collect();
                            src_dirs = Some(dirs);
                            i += 2;
                        } else {
                            anyhow::bail!("Missing value for --src");
                        }
                    }
                    "--config" => {
                        if i + 1 < args.len() {
                            config_path = args[i + 1].clone();
                            i += 2;
                        } else {
                            anyhow::bail!("Missing value for --config");
                        }
                    }
                    "--no-reduce" => {
                        no_reduce = true;
                        i += 1;
                    }
                    _ => {
                        anyhow::bail!("Unknown option: {}", args[i]);
                    }
                }
            }

            Ok(CliArgs {
                cmd: SubCommand::Compile {
                    db_path,
                    src_dirs,
                    config_path,
                    no_reduce,
                },
            })
        }
        "query" => {
            let mut db_path = "knowledge.db".to_string();
            let mut query_string = String::new();

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--db" => {
                        if i + 1 < args.len() {
                            db_path = args[i + 1].clone();
                            i += 2;
                        } else {
                            anyhow::bail!("Missing value for --db");
                        }
                    }
                    _ => {
                        if query_string.is_empty() {
                            query_string = args[i].clone();
                        } else {
                            query_string.push(' ');
                            query_string.push_str(&args[i]);
                        }
                        i += 1;
                    }
                }
            }

            if query_string.is_empty() {
                anyhow::bail!("Query string is required for 'query' subcommand");
            }

            Ok(CliArgs {
                cmd: SubCommand::Query { db_path, query_string },
            })
        }
        "help" | "-h" | "--help" => {
            Ok(CliArgs { cmd: SubCommand::Help })
        }
        _ => {
            anyhow::bail!("Unknown subcommand: {}. Use 'compile', 'query' or 'help'", subcmd);
        }
    }
}

fn print_help() {
    println!("🐾 T.A.N.U.K.I. CLI Tool (Embedded Structured Knowledge DB) 🐾");
    println!("\nUsage:");
    println!("  tanuki <subcommand> [options]");
    println!("\nSubcommands:");
    println!("  compile    Build / update structured knowledge base from documents");
    println!("  query      Search the knowledge base for keywords");
    println!("  help       Print this help message");
    println!("\nCompile Options:");
    println!("  --db <PATH>       Output SQLite DB path (default: knowledge.db)");
    println!("  --src <PATHS>     Scan target directories (comma-separated, default: env/default)");
    println!("  --config <PATH>   Models config JSON path (default: config/models_config.json)");
    println!("  --no-reduce       Skip Reduce (summary aggregation) phase");
    println!("\nQuery Options:");
    println!("  --db <PATH>       Target SQLite DB path (default: knowledge.db)");
    println!("  [QUERY]           Search keywords (space-separated for multi-keywords)");
}

#[tokio::main]
async fn main() -> Result<()> {
    // .env ファイルの自動読み込み (標準ライブラリによる簡易実装)
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                if let Some((key, val)) = line.split_once('=') {
                    let key = key.trim();
                    let mut val = val.trim();
                    if (val.starts_with('"') && val.ends_with('"')) || (val.starts_with('\'') && val.ends_with('\'')) {
                        val = &val[1..val.len() - 1];
                    }
                    if std::env::var(key).is_err() {
                        std::env::set_var(key, val);
                    }
                }
            }
        }
    }

    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("❌ Error parsing arguments: {}", e);
            print_help();
            std::process::exit(1);
        }
    };

    match args.cmd {
        SubCommand::Help => {
            print_help();
            Ok(())
        }
        SubCommand::Query { db_path, query_string } => {
            run_query(&db_path, &query_string)
        }
        SubCommand::Compile {
            db_path,
            src_dirs,
            config_path,
            no_reduce,
        } => {
            let no_reduce_env = std::env::var("TANUKI_NO_REDUCE").map(|v| v == "1").unwrap_or(false);
            let final_no_reduce = no_reduce || no_reduce_env;

            if final_no_reduce {
                println!("T.A.N.U.K.I. Compiler starting... [FAST MODE: Reduce phase skipped]");
            } else {
                println!("T.A.N.U.K.I. Compiler starting...");
            }

            let model_name = std::env::var("TANUKI_MODEL").unwrap_or_else(|_| "gemma4:e4b".to_string());
            println!("  Unified Model Selected: {} (Config: {})", model_name, config_path);

            // configフォルダがない場合は自動作成する
            if let Some(parent) = std::path::Path::new(&config_path).parent() {
                if !parent.exists() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }

            // models_config.jsonが存在しない場合のデフォルト生成フォールバック
            if !std::path::Path::new(&config_path).exists() {
                println!("  ⚠️ Models config file not found. Creating a default offline config for gemma4:e4b...");
                let default_config = serde_json::json!({
                    "gemma4:e4b": {
                        "model_name": "gemma4:e4b",
                        "provider": "Ollama",
                        "display_name": "Gemma 4 Local",
                        "base_url": "http://localhost:11434"
                    }
                });
                if let Ok(json_str) = serde_json::to_string_pretty(&default_config) {
                    let _ = std::fs::write(&config_path, json_str);
                }
            }
            
            let unified_llm = load_provider(&config_path, &model_name)?;

            let res = run_pipeline(final_no_reduce, unified_llm.as_ref(), &db_path, src_dirs).await;

            // VRAMの防衛: 成功・失敗を問わず、必ずOllama等のVRAMモデルをアンロードする
            println!("  🧹 VRAM Guard: Unloading model from GPU/Cloud context...");
            let _ = unified_llm.unload().await;

            res
        }
    }
}

fn run_query(db_path: &str, query_string: &str) -> Result<()> {
    if !Path::new(db_path).exists() {
        anyhow::bail!("Database file '{}' does not exist. Run 'compile' subcommand first.", db_path);
    }
    
    let db = TanukiDb::open(db_path)?;
    let q = query_string.to_lowercase();
    let normalized_q = q.nfkc().collect::<String>();
    let keywords: Vec<&str> = normalized_q.split_whitespace().collect();

    println!("🔍 Searching in '{}' for keywords: {:?}", db_path, keywords);

    let nodes = db.get_all_nodes()?;
    let mut results = Vec::new();

    for node in nodes {
        let title_norm = node.title.to_lowercase().nfkc().collect::<String>();
        let content_norm = node.content.to_lowercase().nfkc().collect::<String>();
        
        let matched = keywords.iter().all(|&kw| {
            title_norm.contains(kw) || content_norm.contains(kw)
        });

        if matched {
            results.push(node);
        }
    }

    if results.is_empty() {
        println!("⚠️ No matching nodes found in database.");
        return Ok(());
    }

    println!("\n✨ Found {} results:", results.len());
    println!("--------------------------------------------------");
    for (i, node) in results.iter().enumerate() {
        println!("[Result {}] 🌲 Node: {}", i + 1, node.title);
        println!("  - Source: file://{}", node.source_path);
        println!("  - Summary: {}", node.summary);
        println!("--------------------------------------------------");
    }

    Ok(())
}

async fn run_pipeline(
    no_reduce: bool,
    unified_llm: &dyn LlmProvider,
    db_path: &str,
    src_dirs: Option<Vec<String>>,
) -> Result<()> {
    // 対象ディレクトリのリスト (引数、環境変数、デフォルトの順に解決)
    let target_dirs_owned: Vec<String> = if let Some(dirs) = src_dirs {
        dirs
    } else if let Ok(dirs_str) = std::env::var("TANUKI_TARGET_DIRS") {
        dirs_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        vec![
            "../../Documents/InBox".to_string(),
            "../../Documents/Archive/Devlog".to_string(),
            "../../Documents/Archive/Media".to_string(),
            "../../Documents/Archive/Specifications".to_string(),
        ]
    };

    let target_dirs: Vec<&str> = target_dirs_owned.iter().map(|s| s.as_str()).collect();

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
    
    let mut db = TanukiDb::open(db_path)?;
    
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
                let path_str = path.to_path_buf().to_string_lossy().to_string();
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
    println!("  Phase 5: Packing AST Knowledge Tree... (Target: {})", db_path);
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
