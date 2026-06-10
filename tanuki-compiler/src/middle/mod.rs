pub mod reducer;
pub mod evaluator;

use tanuki_core::llm::LlmProvider;
use tanuki_core::db::TanukiDb;
use crate::frontend::MdNode;
use anyhow::Result;

pub async fn process_nodes(nodes: Vec<MdNode>, llm: &dyn LlmProvider, db: &mut TanukiDb) -> Result<()> {
    println!("Starting Middle-end processing (Map/Reduce)...");
    
    for node in nodes {
        println!("  Processing node: {}", node.id);
        
        // 1. Embedding 取得 (Gemini)
        let vector = llm.embed(&node.content).await?;
        
        // 2. Metadata 抽出 (Bonsai-8B Optimized Prompt)
        let prompt = format!(
            "### Task: Extract metadata from markdown node.\n### Format: JSON {{\"summary\": \"one sentence\", \"tags\": [\"tag1\", \"tag2\"]}}\n\n### Content:\n{}\n\n### JSON Output:\n",
            node.content
        );
        let metadata_json = llm.generate(&prompt).await?;
        
        // 3. DB 保存
        db.insert_node(
            &node.id,
            &node.source_path,
            node.file_hash.as_deref(),
            &node.context_path,
            &node.title,
            &node.content,
            "AI Summary Placeholder", // summary
            &metadata_json,           // metadata
            &vector                   // embedding
        )?;
        
        println!("    ✓ Stored in knowledge base.");
    }
    
    Ok(())
}

pub use reducer::reduce_knowledge;
