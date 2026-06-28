use tanuki_core::{TanukiDb, Checkpoint, SummaryNode, KnowledgeType, KnowledgeNodeWithEmbedding};
use tanuki_core::schema::{MemoryRootBuilder, ASTNodeBuilder, ConceptVectorBuilder, finish_memory_root_buffer};
use flatbuffers::FlatBufferBuilder;
use std::collections::HashMap;
use std::io::Write;
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

fn calculate_fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn pack_knowledge_base(db: &TanukiDb, output_path: &str) -> Result<()> {
    println!("  Phase 5.5: Packing structured knowledge base into FlatBuffers binary... ({})", output_path);
    
    // 1. DBからすべてのノード（Embedding付き）を取得
    let nodes_with_emb = db.get_all_nodes_with_embedding()?;
    if nodes_with_emb.is_empty() {
        println!("    ⚠️ No nodes found in database. Skipping binary packing.");
        return Ok(());
    }

    // 2. 親・子の木構造をメモリ上に構築
    let mut node_map = HashMap::new();
    let mut parent_to_children: HashMap<u64, Vec<u64>> = HashMap::new();

    for item in &nodes_with_emb {
        let node_id = calculate_fnv1a(&item.node.context_path);
        
        let parent_id = if item.node.context_path.contains(" > ") {
            if let Some(pos) = item.node.context_path.rfind(" > ") {
                let parent_path = &item.node.context_path[..pos];
                calculate_fnv1a(parent_path)
            } else {
                0
            }
        } else {
            0
        };

        node_map.insert(node_id, (item, parent_id));
        if parent_id != 0 {
            parent_to_children.entry(parent_id).or_default().push(node_id);
        }
    }

    // 3. ルートノードを特定
    let mut root_ids = Vec::new();
    for (&node_id, (_, parent_id)) in &node_map {
        if *parent_id == 0 || !node_map.contains_key(parent_id) {
            root_ids.push(node_id);
        }
    }
    root_ids.sort();

    // 4. プリオーダ（深さ優先）走査による順序と子孫・子ノード数の計算
    let mut ordered_nodes = Vec::new();
    let mut child_counts = HashMap::new();
    let mut descendant_counts = HashMap::new();

    fn dfs(
        node_id: u64,
        parent_to_children: &HashMap<u64, Vec<u64>>,
        ordered_nodes: &mut Vec<u64>,
        child_counts: &mut HashMap<u64, u32>,
        descendant_counts: &mut HashMap<u64, u32>,
    ) -> u32 {
        ordered_nodes.push(node_id);
        
        let mut total_descendants = 0;
        if let Some(children) = parent_to_children.get(&node_id) {
            let mut sorted_children = children.clone();
            sorted_children.sort();
            
            child_counts.insert(node_id, sorted_children.len() as u32);
            for child in sorted_children {
                total_descendants += 1 + dfs(child, parent_to_children, ordered_nodes, child_counts, descendant_counts);
            }
        } else {
            child_counts.insert(node_id, 0);
        }
        
        descendant_counts.insert(node_id, total_descendants);
        total_descendants
    }

    for root_id in root_ids {
        dfs(
            root_id,
            &parent_to_children,
            &mut ordered_nodes,
            &mut child_counts,
            &mut descendant_counts,
        );
    }

    // 5. FlatBuffers バイナリの組み立て
    let mut fbb = FlatBufferBuilder::new();
    let mut node_offsets = Vec::new();

    for &node_id in &ordered_nodes {
        let (item, parent_id) = node_map.get(&node_id).unwrap();
        let child_count = *child_counts.get(&node_id).unwrap_or(&0);
        let descendant_count = *descendant_counts.get(&node_id).unwrap_or(&0);

        // Concept Vector (Embedding)
        let mut v_array = [0.0f32; 768];
        let limit = std::cmp::min(item.embedding.len(), 768);
        v_array[..limit].copy_from_slice(&item.embedding[..limit]);
        let v_offset = fbb.create_vector(&v_array);

        let mut cv_builder = ConceptVectorBuilder::new(&mut fbb);
        cv_builder.add_v(v_offset);
        let cv = cv_builder.finish();

        let title_offset = fbb.create_string(&item.node.title);
        let logic_offset = fbb.create_string(&item.node.content);

        let mut node_builder = ASTNodeBuilder::new(&mut fbb);
        node_builder.add_node_id(node_id);
        node_builder.add_parent_id(*parent_id);
        node_builder.add_child_count(child_count);
        node_builder.add_descendant_count(descendant_count);
        node_builder.add_title(title_offset);
        node_builder.add_concept(cv);
        node_builder.add_raw_logic(logic_offset);

        node_offsets.push(node_builder.finish());
    }

    let nodes_vector_offset = fbb.create_vector(&node_offsets);

    let mut root_builder = MemoryRootBuilder::new(&mut fbb);
    root_builder.add_version(1);
    root_builder.add_active_nodes(nodes_vector_offset);
    let root_offset = root_builder.finish();

    finish_memory_root_buffer(&mut fbb, root_offset);

    // 6. ファイルへの書き出し
    let mut file = fs::File::create(output_path)?;
    file.write_all(fbb.finished_data())?;
    println!("  ✓ Packed structured knowledge binary saved to: {}", output_path);

    Ok(())
}
