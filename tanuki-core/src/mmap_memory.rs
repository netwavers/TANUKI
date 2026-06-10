use anyhow::{Result, Context};
use arc_swap::ArcSwap;
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use crate::schema::get_root_as_memory_root;

pub struct MmapMemoryManager {
    // RCU-based active memory mapping
    active_mmap: ArcSwap<Mmap>,
}

impl MmapMemoryManager {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path).context("Failed to open memory file")?;
        let mmap = unsafe { Mmap::map(&file) }.context("Failed to mmap file")?;
        Ok(Self {
            active_mmap: ArcSwap::from_pointee(mmap),
        })
    }

    pub fn update_mapping<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::open(path).context("Failed to open memory file for update")?;
        let mmap = unsafe { Mmap::map(&file) }.context("Failed to mmap file")?;
        self.active_mmap.store(Arc::new(mmap));
        Ok(())
    }

    pub fn search(&self, query_vector: &[f32; 768], top_k: usize) -> Result<Vec<(u64, f32)>> {
        let mmap_guard = self.active_mmap.load();
        let mmap_data = &**mmap_guard;

        let root = get_root_as_memory_root(mmap_data);
        let nodes = root.active_nodes().context("No active nodes found")?;

        let mut coarse_results = Vec::with_capacity(nodes.len());

        // Coarse Search (First 64 dimensions) with Subtree Skip Optimization
        let mut i = 0;
        while i < nodes.len() {
            let node = nodes.get(i);
            let mut final_score = 0.0;
            
            if let Some(concept) = node.concept() {
                if let Some(v_vec) = concept.v() {
                    // MRL: First 64 dimensions
                    let mut score = 0.0;
                    let mut norm_a = 0.0;
                    let mut norm_b = 0.0;
                    
                    for j in 0..64 {
                        let val_b = v_vec.get(j);
                        let val_a = query_vector[j];
                        score += val_a * val_b;
                        norm_a += val_a * val_a;
                        norm_b += val_b * val_b;
                    }
                    
                    final_score = if norm_a == 0.0 || norm_b == 0.0 {
                        0.0
                    } else {
                        score / (norm_a.sqrt() * norm_b.sqrt())
                    };
                }
            }

            // 意味境界（類似度しきい値 0.25 未満）によるサブツリースキップ
            let descendant_count = node.descendant_count();
            if final_score < 0.25 && descendant_count > 0 {
                // 子孫ノードを丸ごとスキップして次の兄弟ノードへジャンプ
                i += 1 + descendant_count as usize;
            } else {
                coarse_results.push((i, final_score));
                i += 1;
            }
        }

        // Sort by coarse score descending
        coarse_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Fine Ranking (Full 768 dimensions for top N)
        let refine_count = std::cmp::min(coarse_results.len(), top_k * 5);
        let mut fine_results = Vec::with_capacity(refine_count);

        for (idx, _) in coarse_results.into_iter().take(refine_count) {
            let node = nodes.get(idx);
            if let Some(concept) = node.concept() {
                if let Some(v_vec) = concept.v() {
                    let mut score = 0.0;
                    let mut norm_a = 0.0;
                    let mut norm_b = 0.0;
                    
                    // Full 768 dimensions
                    let limit = std::cmp::min(v_vec.len(), 768);
                    for j in 0..limit {
                        let val_b = v_vec.get(j);
                        let val_a = query_vector[j];
                        score += val_a * val_b;
                        norm_a += val_a * val_a;
                        norm_b += val_b * val_b;
                    }
                    
                    let final_score = if norm_a == 0.0 || norm_b == 0.0 {
                        0.0
                    } else {
                        score / (norm_a.sqrt() * norm_b.sqrt())
                    };
                    
                    fine_results.push((node.node_id(), final_score));
                }
            }
        }

        fine_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        fine_results.truncate(top_k);

        Ok(fine_results)
    }
}
