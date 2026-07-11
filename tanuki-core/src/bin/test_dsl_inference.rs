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
    let args: Vec<String> = env::args().collect();
    let is_prune_test = args.contains(&"--prune-test".to_string());

    if is_prune_test {
        println!("🐾 Flat-AST DSL LLM Integration Test [Pruning Stress Mode] 🐾");
    } else {
        println!("🐾 Flat-AST DSL LLM Integration Test [Standard Mode] 🐾");
    }

    // 1. インメモリDBの作成とテスト知識ノードの登録
    println!("  [Step 1] Initializing in-memory TanukiDb...");
    let db = TanukiDb::open(":memory:")?;

    // 本物の知識ノード
    let real_node_id = "a1b2c3d4";
    db.insert_node(
        real_node_id,
        "src/lin_music.rs",
        None,
        "root/music",
        "平沢リンの好きな高中正義の曲",
        "平沢リン（キャラクター）は80〜90年代の音楽、特に高中正義の『BLUE LAGOON』という曲が大好きです。",
        "平沢リンは高中正義のBLUE LAGOONが好き",
        "{}",
        &[],
    )?;

    // 限界負荷テストの場合は、無関係なダミーノードを大量に投入 (20ノード分、約3000トークン)
    if is_prune_test {
        println!("  [Step 1b] Injecting 20 low-priority dummy knowledge nodes into database...");
        for i in 1..=20 {
            let dummy_id = format!("{:08x}", i + 1000); // "000003e9" などの8桁16進ハッシュ
            let dummy_content = format!(
                "これはダミーのギタリスト紹介データ第{}番です。ライブでのアンプの設定や、予備のギター弦のメーカー、ツアーでの楽屋の弁当の好みなど、今回の曲名の特定タスクには一切関係のない膨大なノイズデータが書かれています。",
                i
            );
            db.insert_node(
                &dummy_id,
                "src/dummy_gear.rs",
                None,
                "root/dummy",
                &format!("Dummy Info {}", i),
                &dummy_content,
                "Dummy Summary",
                "{}",
                &[],
            )?;
        }
    }

    // 2. Checkpoint (セッション状態) の構築
    println!("  [Step 2] Constructing mock Checkpoint...");
    let mut checkpoint = Checkpoint::new("test-session".to_string(), "hash123".to_string());

    // システム制約 (SystemNode) の追加 - 絶対保護 (Priority = 0)
    checkpoint.context_stack.active_constraints.push(
        "回答は必ず『〜ですわ』というお嬢様メイド口調で終わるようにし、結論の曲名だけを日本語で簡潔に答えなさい。".to_string()
    );

    // 目標 (InstructionNode) の追加 - 絶対保護 (Priority = 0)
    checkpoint.context_stack.spec_driven_goals.push(SpecGoal {
        id: "平沢リンが最も好きな高中正義の曲の名前を答えなさい。".to_string(),
        status: "PENDING".to_string(),
        priority: 5,
    });

    // 知識グラフサマリー (ReferenceNode) の追加
    // 本物のノードは hot_zones に登録して優先度 1 に引き上げる
    checkpoint.knowledge_graph_summary.nodes.push(SummaryNode {
        id: real_node_id.to_string(),
        node_type: KnowledgeType::Node,
        symbol: "lin_favorite_song".to_string(),
        link: "src/lin_music.rs".to_string(),
    });
    checkpoint
        .knowledge_graph_summary
        .hot_zones
        .push(real_node_id.to_string());

    // 限界負荷テスト時のダミーノードのバインド (これらは hot_zones に入れないため、デフォルト優先度 5 となる)
    if is_prune_test {
        for i in 1..=20 {
            let dummy_id = format!("{:08x}", i + 1000);
            checkpoint.knowledge_graph_summary.nodes.push(SummaryNode {
                id: dummy_id,
                node_type: KnowledgeType::Node,
                symbol: format!("dummy_node_{}", i),
                link: "src/dummy_gear.rs".to_string(),
            });
        }
    }

    // 3. Checkpoint から FlatAST へトランスパイル & DSL レンダリング
    println!("  [Step 3] Transpiling Checkpoint to FlatAST...");
    let mut ast = checkpoint.to_flat_ast(&db)?;

    // 限界負荷テスト時は、プルーニングを発火させる
    if is_prune_test {
        let initial_tokens = ast.total_tokens();
        println!(
            "  [Step 3b] Pruning stress test triggers. Initial tokens count: {}",
            initial_tokens
        );

        // トークン上限を極小の 400 に絞り、プルーニングを実行
        let target_limit = 400;
        let pruned_tokens = ast.prune(target_limit);
        println!(
            "            Pruned tokens count (target 400): {}",
            pruned_tokens
        );
        if pruned_tokens > target_limit {
            eprintln!("            ⚠️  [Flat-AST WARNING]: Budget Auto-Expand Triggered! Target limit was {} but expanded to {} due to absolute protected nodes.", target_limit, pruned_tokens);
        }

        // プルーニングのアサーション
        assert!(
            pruned_tokens <= target_limit,
            "FAILED: Pruned token count ({}) exceeded the limit of 400",
            pruned_tokens
        );
        println!("            ✅ Token Limit Assertion: PASSED (Under 400 tokens limit)");

        let dsl_temp = ast.render_dsl();
        assert!(
            dsl_temp.contains("BLUE LAGOON"),
            "FAILED: Critical knowledge 'BLUE LAGOON' was mistakenly pruned!"
        );
        println!("            ✅ Critical Knowledge Safety Assertion: PASSED (BLUE LAGOON was preserved)");

        // テスト 2: 極小予算自動拡張テスト（上限 100）
        println!("  [Step 3c] Triggering over-budget auto-expand test (target limit 100)...");
        let mut ast_auto = checkpoint.to_flat_ast(&db)?;
        let target_limit_auto = 100;
        let pruned_auto = ast_auto.prune(target_limit_auto);
        println!(
            "            Pruned tokens count (target 100, absolute protected is 234): {}",
            pruned_auto
        );
        if pruned_auto > target_limit_auto {
            eprintln!("            ⚠️  [Flat-AST WARNING]: Budget Auto-Expand Triggered! Target limit was {} but expanded to {} due to absolute protected nodes.", target_limit_auto, pruned_auto);
        }

        assert_eq!(
            pruned_auto, 234,
            "FAILED: Pruned token count for auto-expand should be exactly 234, but got {}",
            pruned_auto
        );
        println!("            ✅ Auto-Expand Limit Assertion: PASSED (Returned exactly 234)");

        let dsl_auto = ast_auto.render_dsl();
        assert!(dsl_auto.contains("お嬢様メイド口調"));
        assert!(dsl_auto.contains("高中正義の曲の名前を答えなさい。"));
        assert!(!dsl_auto.contains("BLUE LAGOON"));
        println!("            ✅ Auto-Expand Content Selection Assertion: PASSED (Protected nodes safe, low priority node pruned)");
    }

    let dsl = ast.render_dsl();
    let human_doc = ast.render_human_readable();

    println!("\n--- Generated Flat-AST Human-Readable Document ---");
    print!("{}", human_doc);
    println!("---------------------------------------------------\n");

    println!("--- Generated Flat-AST DSL Prompt ---");
    print!("{}", dsl);
    println!("--------------------------------------\n");

    // 4. Ollama クライアントの準備
    let model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:latest".to_string());
    let base_url = "http://localhost:11434".to_string();
    println!("  [Step 4] Initializing OllamaClient (Model: {})...", model);
    let provider = OllamaClient::new(base_url, model);

    // 5. LLM 推論の実行
    println!("  [Step 5] Sending DSL prompt to local Ollama...");
    let response = provider.generate(&dsl).await?;

    println!("\n--- LLM Response ---");
    println!("{}", response);
    println!("--------------------\n");

    // 6. 適合性適合テスト（アサーション）
    println!("  [Step 6] Running compatibility assertions...");

    // 知識アサーション: 知識ベース内の "BLUE LAGOON" が含まれているか
    let contains_knowledge = response.contains("BLUE LAGOON")
        || response.contains("blue lagoon")
        || response.contains("Blue Lagoon");
    assert!(
        contains_knowledge,
        "FAILED: LLM response does not contain the expected knowledge 'BLUE LAGOON'"
    );
    println!("   ✅ Knowledge Reference Assertion: PASSED (Contains 'BLUE LAGOON')");

    // 口調制約アサーション: お嬢様メイド口調「ですわ」を守っているか
    let respects_persona = response.contains("ですわ");
    assert!(
        respects_persona,
        "FAILED: LLM response does not respect the constraint ('ですわ' お嬢様メイド口調)"
    );
    println!("   ✅ System Constraint Assertion: PASSED (Ended with / contains 'ですわ')");

    // DSL露出防御アサーション: プロンプト内の DSL 記号 (#S:, #I:, #R:) をそのまま出力していないか
    let leaks_dsl =
        response.contains("#S:") || response.contains("#I:") || response.contains("#R:");
    assert!(
        !leaks_dsl,
        "FAILED: LLM leaked DSL markers (#S:, #I:, #R:) in its response"
    );
    println!("   ✅ DSL Isolation Assertion: PASSED (DSL markers not leaked)");

    println!("\n🐾 ALL DSL INTEGRATION TESTS PASSED PERFECTLY! 🐾");
    Ok(())
}
