use tanuki_core::llm::LlmProvider;
use tanuki_core::db::{TanukiDb, KnowledgeNode};
use anyhow::Result;
use std::collections::HashMap;

pub async fn reduce_knowledge(llm: &dyn LlmProvider, db: &mut TanukiDb) -> Result<()> {
    println!("🐾 Starting Reduction (Recursive Summarization)...");
    
    // 1. 全ノードを取得
    let nodes = db.get_all_nodes()?;
    if nodes.is_empty() {
        println!("  ⚠️ No nodes found in knowledge base. Skipping Reduction.");
        return Ok(());
    }
    
    // 2. ContextPath でグルーピング
    // 階層的な構造をフラットに扱いつつ、同じコンテキストのものをまとめる
    let mut groups: HashMap<String, Vec<KnowledgeNode>> = HashMap::new();
    for node in nodes {
        groups.entry(node.context_path.clone()).or_default().push(node);
    }
    
    // 3. 各グループを要約して Cluster を作成
    for (path, group_nodes) in groups {
        println!("  Reducing context group: {}", path);
        
        let combined_content: String = group_nodes.iter()
            .map(|n| format!("- Title: {}\n  Content: {}", n.title, n.content))
            .collect::<Vec<_>>()
            .join("\n\n");
            
        let prompt = format!(
            "Before providing the final answer, please think step-by-step and output your logical process within <thinking> tags.\n\
             These are information snippets under the context path '{}'. \
             Summarize them into a coherent 'Knowledge Cluster'. \
             Provide a clear title and a summary that explains how these items relate to each other.\n\n\
             Nodes:\n{}",
             path, combined_content
        );
        
        let result = llm.generate(&prompt).await?;
        
        // Thinking タグを除去
        let clean_result = if let Some(pos) = result.rfind("</thinking>") {
            result[pos + 11..].trim().to_string()
        } else {
            result.clone()
        };

        // 簡単なパース（タイトルと要約を分離。実際には JSON 出力が望ましい）
        let (title, summary) = if let Some((t, s)) = clean_result.split_once('\n') {
            (t.trim().trim_start_matches("Title:").trim().to_string(), s.trim().to_string())
        } else {
            (format!("Cluster: {}", path), clean_result)
        };
        
        // 4. Cluster テーブルに保存
        db.insert_cluster(&path, "root", &title, &summary, "Standard context navigation")?;
        println!("    ✓ Cluster created: {}", title);
    }
    
    println!("🐾 Reduction complete.");
    Ok(())
}
