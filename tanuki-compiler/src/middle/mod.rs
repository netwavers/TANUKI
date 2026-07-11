pub mod reducer;
pub mod evaluator;

use tanuki_core::llm::LlmProvider;
use tanuki_core::db::TanukiDb;
use crate::frontend::MdNode;
use anyhow::Result;

pub async fn process_nodes(nodes: Vec<MdNode>, llm: &dyn LlmProvider, db: &mut TanukiDb) -> Result<()> {
    println!("Starting Middle-end processing (Map/Reduce)...");
    
    // Phase A: Generate Embeddings in batch (holds embedding model in VRAM)
    println!("  Phase A: Generating Embeddings for {} nodes...", nodes.len());
    let mut vectors = Vec::with_capacity(nodes.len());
    for (i, node) in nodes.iter().enumerate() {
        if i % 10 == 0 || i == nodes.len() - 1 {
            println!("    Embedding progress: {}/{}", i + 1, nodes.len());
        }
        let vector = llm.embed(&node.content).await?;
        vectors.push(vector);
    }
    
    // Phase B: Generate Metadata and Store in batch (holds generation model in VRAM)
    println!("  Phase B: Generating Metadata and storing {} nodes...", nodes.len());
    for (i, node) in nodes.into_iter().enumerate() {
        println!("    Processing node {}/{}: {}", i + 1, vectors.len(), node.id);
        
        // 2. Metadata 抽出 (Bonsai-8B Optimized Prompt)
        // ※ 高速ビルドテスト時は、以下のダミーJSONに差し替えることでOllama推論をスキップ可能です:
        // let metadata_json = "{\"summary\": \"\", \"tags\": []}".to_string();
        let prompt = format!(
            "### Task: Extract metadata from markdown node.\n### Format: JSON {{\"summary\": \"one sentence\", \"tags\": [\"tag1\", \"tag2\"]}}\n\n### Content:\n{}\n\n### JSON Output:\n",
            node.content
        );
        let metadata_json = llm.generate(&prompt).await?;
        let vector = &vectors[i];
        
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
            vector                   // embedding
        )?;
        
        println!("    ✓ Stored in knowledge base.");
    }
    
    Ok(())
}

pub use reducer::reduce_knowledge;
