use tanuki_core::schema::{MemoryRootBuilder, ASTNodeBuilder, ConceptVectorBuilder, finish_memory_root_buffer};
use tanuki_core::MmapMemoryManager;
use flatbuffers::FlatBufferBuilder;
use std::fs::File;
use std::io::Write;

fn main() -> anyhow::Result<()> {
    println!("🧪 Testing Irminsul V5 (Pre-order Tree & Subtree Skip Optimization)...");

    let path = "test_memory.bin";
    
    // 1. テストデータの作成 (FlatBuffers)
    let mut fbb = FlatBufferBuilder::new();
    
    let mut node_offsets = Vec::new();
    
    // 木構造テストデータ定義：
    // Node 0 (Root) [descendants: 5, children: 2]
    //   Node 1 (Child A) [descendants: 2, children: 2]
    //     Node 2 (Grandchild A1) [descendants: 0, children: 0]
    //     Node 3 (Grandchild A2) [descendants: 0, children: 0]
    //   Node 4 (Child B) [descendants: 1, children: 1]
    //     Node 5 (Grandchild B1) [descendants: 0, children: 0]
    // Node 6 (Root 2) [descendants: 1, children: 1]
    //   Node 7 (Child C) [descendants: 0, children: 0]
    let test_nodes = vec![
        // (id, parent_id, child_count, descendant_count, title)
        (0, 0, 2, 5, "Root 1"),
        (1, 0, 2, 2, "Child A"),
        (2, 1, 0, 0, "Grandchild A1"),
        (3, 1, 0, 0, "Grandchild A2"),
        (4, 0, 1, 1, "Child B"),
        (5, 4, 0, 0, "Grandchild B1"),
        (6, 6, 1, 1, "Root 2"),
        (7, 6, 0, 0, "Child C"),
    ];
    
    for (i, parent_id, child_count, descendant_count, title_str) in test_nodes {
        let mut v = [0.0f32; 768];
        // i番目の次元に1.0を立てる
        if i < 768 {
            v[i] = 1.0;
        }
        
        // 探索パス疎通のための類似度重み付け (スキップしきい値 0.25 を越えるようにする)
        if i == 0 {
            v[5] = 0.4; // Node 5 のクエリを Root 1 でスキップさせない
            v[2] = 0.4; // Node 2 のクエリを Root 1 でスキップさせない
        }
        if i == 4 {
            v[5] = 0.4; // Node 5 のクエリを Child B でスキップさせない
        }
        
        let v_offset = fbb.create_vector(&v);
        
        let mut cv_builder = ConceptVectorBuilder::new(&mut fbb);
        cv_builder.add_v(v_offset);
        let cv = cv_builder.finish();
        
        let logic = fbb.create_string(&format!("Node Logic {}", i));
        let title = fbb.create_string(title_str);
        
        let mut node_builder = ASTNodeBuilder::new(&mut fbb);
        node_builder.add_node_id(i as u64);
        node_builder.add_parent_id(parent_id as u64);
        node_builder.add_child_count(child_count);
        node_builder.add_descendant_count(descendant_count);
        node_builder.add_title(title);
        node_builder.add_concept(cv);
        node_builder.add_raw_logic(logic);
        node_offsets.push(node_builder.finish());
    }
    
    let nodes_vec = fbb.create_vector(&node_offsets);
    
    let mut root_builder = MemoryRootBuilder::new(&mut fbb);
    root_builder.add_version(1);
    root_builder.add_active_nodes(nodes_vec);
    let root = root_builder.finish();
    
    finish_memory_root_buffer(&mut fbb, root);
    
    let mut file = File::create(path)?;
    file.write_all(fbb.finished_data())?;
    println!("  ✅ Test binary created: {}", path);

    // 2. MmapMemoryManagerでの読み込みと検索
    let manager = MmapMemoryManager::new(path)?;
    println!("  ✅ Mmap mapping successful.");

    // テスト1: 直交概念でNode 5を探す（Child B配下。スキップされず見つかるか？）
    let mut query_5 = [0.0f32; 768];
    query_5[5] = 1.0; 
    let results_5 = manager.search(&query_5, 3)?;
    
    println!("🔍 Search Results for Node 5 (Should find 5):");
    for (id, score) in &results_5 {
        println!("  - Node ID: {}, Score: {:.4}", id, score);
    }
    assert!(!results_5.is_empty() && results_5[0].0 == 5);
    println!("  ✅ Success: Node 5 was successfully searched!");

    // テスト2: スキップ機能の検証
    // クエリは Node 2 (Grandchild A1) を探す。
    // しかし、探索中に Node 1 (Child A) とのコサイン類似度がしきい値(0.25)未満になるように、
    // 親である Node 1 への類似度が極めて低くなるクエリを与える。
    // クエリ: query[2]=1.0 (Grandchild A1のベクトルは直交しているため、親である Node 1 (query[1]=1.0) とのコサイン類似度は 0.0 になる)
    // このとき、Node 1 に差し掛かった時点で類似度 0.0 < 0.25 と判定され、
    // descendant_count = 2 により、Node 2 (A1) と Node 3 (A2) を含むサブツリー全体が飛び越される（スキップされる）はず！
    // 結果、Node 2 は探索されず、スコア計算も行われないため、検索結果に入らない。
    let mut query_skip = [0.0f32; 768];
    query_skip[2] = 1.0; // 本来は Node 2 を探したいが、親の Node 1 と直交
    
    let results_skip = manager.search(&query_skip, 3)?;
    println!("🔍 Search Results for Query 2 (A1) with skip enabled (Should NOT find 2):");
    for (id, score) in &results_skip {
        println!("  - Node ID: {}, Score: {:.4}", id, score);
    }
    
    // スキップされたため、Node 2 も Node 3 もヒットせず、Node 0 や他のノードのみが低スコアで残るはず
    let found_2 = results_skip.iter().any(|(id, _)| *id == 2);
    if !found_2 {
        println!("  ✅ Success: Node 2 (Grandchild A1) was correctly skipped because its parent Node 1 was irrelevant!");
    } else {
        println!("  ❌ Failure: Node 2 was searched despite its parent being irrelevant.");
    }

    Ok(())
}
