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
use std::fs;
use tanuki_core::llm::OllamaClient;
use tanuki_core::LlmProvider;
use tanuki_core::{calculate_fnv1a, flat_ast::PriorityTier, FlatAST};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🐾 Flat-AST README.md Fallback & Cost Reduction Test 🐾");

    // 1. README.md の読み込み
    let readme_path = "D:/Projects/PyProjects/TANUKI/tanuki-core/README.md";
    println!("  [Step 1] Reading README.md from: {} ...", readme_path);
    let readme_content = fs::read_to_string(readme_path)?;

    // 2. セクション分割処理
    let mut sections = Vec::new();
    let mut current_title = "Header".to_string();
    let mut current_body = String::new();

    for line in readme_content.lines() {
        if line.starts_with("# ") || line.starts_with("## ") || line.starts_with("### ") {
            if current_title != "Header" {
                sections.push((current_title.clone(), current_body.clone()));
            }
            current_title = line.to_string();
            current_body = String::new();
        } else {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }
    if current_title != "Header" {
        sections.push((current_title, current_body));
    }

    println!(
        "            Parsed {} sections from README.md",
        sections.len()
    );

    // 3. FlatAST への投入
    let mut ast = FlatAST::new();

    // 絶対保護する SystemNode (制約指示)
    let system_constraint = "回答は必ず『〜ですわ』というお嬢様メイド口調で終わるようにし、結論のパディングバイト数だけを日本語で簡潔に答えなさい。";
    let task_instruction = "README.md の仕様定義を参考に、tanuki-core がアライメント境界として採用している物理メモリのパディングバイト数を答えなさい。";

    ast.push_node(
        calculate_fnv1a(system_constraint),
        0,
        0,
        false,
        0,
        system_constraint,
    );
    ast.push_node(
        calculate_fnv1a(task_instruction),
        1,
        0,
        false,
        0,
        task_instruction,
    );

    // 親ノード (README.md リファレンス)
    let parent_id = calculate_fnv1a("README.md");
    ast.push_node(
        parent_id,
        2,
        1,
        false,
        sections.len() as u16,
        "tanuki-core: Flat-AST Context Architecture README.md",
    );

    // 各セクションを SubNode として追加
    for (title, body) in &sections {
        let payload = format!("{}\n{}", title, body);
        let node_id = calculate_fnv1a(&payload);

        // 重要度の割り当て
        let tier = if title.contains("概要") || title.contains("メモリ") || title.contains("API")
        {
            PriorityTier::Tier1
        } else if title.contains("DSL") || title.contains("プルーニング") {
            PriorityTier::Tier2
        } else {
            PriorityTier::Tier3 // 運用監視や検証方法など
        };

        println!("            Section: '{}' -> Tier {:?}", title, tier);
        ast.push_node(node_id, 2, tier as u8, true, 0, &payload);
    }

    let initial_tokens = ast.total_tokens();
    println!(
        "            Initial README.md Flat-AST tokens: {} bytes",
        initial_tokens
    );

    // 4. 第一段階のプルーニング（Tier3 の削除検証）
    // target_limit を 60% 程度に絞り、Tier3 を削る
    let limit_tier3 = (initial_tokens as f64 * 0.6) as u32;
    println!(
        "  [Step 2] Pruning to target limit ({}): removing Tier3...",
        limit_tier3
    );
    let pruned_tier3 = ast.prune(limit_tier3);
    println!("            Pruned tokens: {} bytes", pruned_tier3);

    let dsl_1 = ast.render_dsl();
    println!(
        "--- Debug dsl_1 output ---\n{}\n--------------------------",
        dsl_1
    );

    // アサーション：Tier3 が消えているか
    let contains_tier3 = dsl_1.contains("監視") || dsl_1.contains("検証");
    assert!(!contains_tier3, "FAILED: Tier3 subnodes were not pruned!");

    // アサーション：Tier1 が残っているか
    let contains_tier1 = dsl_1.contains("メモリ") && dsl_1.contains("API");
    assert!(
        contains_tier1,
        "FAILED: Tier1 subnodes were mistakenly pruned!"
    );
    println!("            ✅ Tier3 Pruning Assertion: PASSED (Tier 3 pruned, Tier 1 preserved)");

    // 5. 第二段階のプルーニング（Tier2 の削除検証）
    // target_limit を 30% 程度にして Tier2 も削る
    let limit_tier2 = (initial_tokens as f64 * 0.3) as u32;
    println!(
        "  [Step 3] Pruning further to target limit ({}): removing Tier2...",
        limit_tier2
    );
    let pruned_tier2 = ast.prune(limit_tier2);
    println!("            Pruned tokens: {} bytes", pruned_tier2);

    let dsl_2 = ast.render_dsl();
    // アサーション：Tier2 が消えているか
    let contains_tier2 = dsl_2.contains("DSL") || dsl_2.contains("プルーニング");
    assert!(!contains_tier2, "FAILED: Tier2 subnodes were not pruned!");

    // アサーション：Tier1 が残っているか
    let contains_tier1_v2 = dsl_2.contains("メモリ") && dsl_2.contains("API");
    assert!(
        contains_tier1_v2,
        "FAILED: Tier1 subnodes were mistakenly pruned in stage 2!"
    );
    println!("            ✅ Tier2 Pruning Assertion: PASSED (Tier 2 pruned, Tier 1 preserved)");

    // 6. Ollama 結合テスト（実推論）
    let model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| "gemma4:e2b".to_string());
    let base_url = "http://localhost:11434".to_string();
    println!(
        "  [Step 4] Initializing OllamaClient (Model: {}, URL: {})...",
        model, base_url
    );
    let provider = OllamaClient::new(base_url, model);

    println!("  [Step 5] Sending pruned DSL prompt (Tier 3 & 2 removed) to local Ollama...");
    let response = provider.generate(&dsl_2).await?;

    println!("\n--- LLM Response ---");
    println!("{}", response);
    println!("--------------------\n");

    // 7. 実推論回答およびアサーションの検証
    // アライメントバイト数「16」が含まれていること
    let contains_answer = response.contains("16") || response.contains("１６");
    assert!(
        contains_answer,
        "FAILED: LLM response does not contain the expected answer '16 bytes'"
    );

    // 制約「ですわ」お嬢様メイド口調であること
    let respects_persona = response.contains("ですわ");
    assert!(
        respects_persona,
        "FAILED: LLM did not respect the persona constraint ('ですわ' お嬢様メイド口調)"
    );

    // DSLがリークしていないこと
    let leaks_dsl =
        response.contains("#S:") || response.contains("#I:") || response.contains("#R:");
    assert!(!leaks_dsl, "FAILED: DSL leaked in LLM response");

    println!("   ✅ Knowledge Reference Assertion: PASSED (Contains '16')");
    println!("   ✅ System Constraint Assertion: PASSED (Ended with / contains 'ですわ')");
    println!("   ✅ DSL Isolation Assertion: PASSED (DSL markers not leaked)");

    println!("\n🐾 ALL README FALLBACK INTEGRATION TESTS PASSED PERFECTLY! 🐾");
    Ok(())
}
