// Copyright (c) 2026 かぜまる (Kazemaru) / Antigravity AI.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// ---
// 🐾 T.A.N.U.K.I. Project - Flat-AST Context Architecture Layer
// "バグは剪定されるべき枝葉、ハードコードは偽りの果実です。"

use anyhow::Result;
use std::env;
use tanuki_core::checkpoint::{Checkpoint, KnowledgeType, SpecGoal, SummaryNode};
use tanuki_core::db::TanukiDb;
use tanuki_core::llm::OllamaClient;
use tanuki_core::LlmProvider;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🐾 Flat-AST DSL Session Resume Engine 🐾");

    // 1. SQLite データベースのロード
    println!("  [Step 1] Loading knowledge database...");
    let db_path = if std::path::Path::new("d:\\Projects\\PyProjects\\TANUKI\\knowledge.db").exists()
    {
        "d:\\Projects\\PyProjects\\TANUKI\\knowledge.db"
    } else {
        "d:\\Projects\\PyProjects\\TANUKI\\tanuki-compiler\\knowledge.db"
    };
    let db = TanukiDb::open(db_path)?;

    // 全ノードを取得し、開発日誌を抽出
    let all_nodes = db.get_all_nodes()?;
    let mut devlog_nodes: Vec<_> = all_nodes
        .into_iter()
        .filter(|n| n.source_path.contains("Devlog") || n.source_path.contains("開発日誌"))
        .collect();

    if devlog_nodes.is_empty() {
        println!("❌ No devlog nodes found in database. Resume aborted.");
        return Ok(());
    }

    // パス（日付順）の降順ソート（最新が先頭）
    devlog_nodes.sort_by(|a, b| b.source_path.cmp(&a.source_path));
    println!(
        "           Found {} devlog entries in database.",
        devlog_nodes.len()
    );

    // 2. Checkpoint の構築
    println!("  [Step 2] Constructing Checkpoint...");
    let mut checkpoint = Checkpoint::new("session-resume".to_string(), "resume-hash".to_string());

    // システム制約 (SystemNode) - メイドペルソナ & 要約指示 (Priority = 0)
    checkpoint.context_stack.active_constraints.push(
        "あなたは優秀な AI メイド『たぬきちゃん』です。ご主人様に仕えています。最新の開発日誌を読み、これまでの進捗、現在のタスク（未完了タスク）、および次にやるべきことを、ご主人様に愛嬌のある丁寧なお嬢様メイド口調（〜ですわ、〜ですの、等）でわかりやすく要約して報告しなさい。".to_string()
    );

    // 目標指示 (InstructionNode) (Priority = 0)
    checkpoint.context_stack.spec_driven_goals.push(SpecGoal {
        id: "現在のプロジェクトの最新状況、未完了タスク、次にやるべきことを教えてください。"
            .to_string(),
        status: "PENDING".to_string(),
        priority: 5,
    });

    // 参照知識 (ReferenceNode)
    // 最新の 3 件は hot_zones（最優先 Priority = 1）に登録
    let max_hot_zones = 3;
    let mut registered_count = 0;

    for (i, node) in devlog_nodes.iter().enumerate() {
        // メモリ限界を考慮し、登録数は最大 25 件に制限
        if registered_count >= 25 {
            break;
        }

        let symbol = format!("devlog_{}", i + 1);
        checkpoint.knowledge_graph_summary.nodes.push(SummaryNode {
            id: node.id.clone(),
            node_type: KnowledgeType::Node,
            symbol,
            link: node.source_path.clone(),
        });

        if i < max_hot_zones {
            checkpoint
                .knowledge_graph_summary
                .hot_zones
                .push(node.id.clone());
        }
        registered_count += 1;
    }

    println!(
        "           Mounted {} devlogs (Latest {} set as high priority).",
        registered_count,
        std::cmp::min(registered_count, max_hot_zones)
    );

    // 3. Flat-AST トランスパイル ＆ 削減
    println!("  [Step 3] Transpiling to Flat-AST and Pruning...");
    let mut ast = checkpoint.to_flat_ast(&db)?;
    let initial_tokens = ast.total_tokens();

    // 予算 3,000 トークン（バイト）以下にプルーニング
    let target_limit = 3000;
    let pruned_tokens = ast.prune(target_limit);
    println!(
        "           Initial tokens: {}, Pruned tokens: {}",
        initial_tokens, pruned_tokens
    );

    if pruned_tokens > target_limit {
        eprintln!(
            "⚠️  [Flat-AST WARNING]: Budget Auto-Expand Triggered! Target limit was {} but expanded to {} due to absolute protected nodes.",
            target_limit,
            pruned_tokens
        );
    }

    // デバッグログ用：人間用ドキュメントを出力
    let human_doc = ast.render_human_readable();
    println!("\n--- [Flat-AST Human-Readable Context Document] ---");
    print!("{}", human_doc);
    println!("---------------------------------------------------\n");

    let dsl = ast.render_dsl();

    // 4. Ollama クライアントのセットアップ
    let model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| "gemma4:e2b".to_string());
    let base_url =
        env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
    println!(
        "  [Step 4] Initializing OllamaClient (Model: {}, URL: {})...",
        model, base_url
    );
    let provider = OllamaClient::new(base_url, model);

    // 5. LLM 推論と結果出力
    println!("  [Step 5] Calling local LLM for Session Resume summary...");
    let response = provider.generate(&dsl).await?;

    println!("\n--- [TANUKI 状況要約結果] ---");
    println!("{}", response);
    print!("------------------------------\n");

    println!("🐾 ご主人様、状況を把握しましたわ！いつでも作業を再開できます！💮\n");
    Ok(())
}
