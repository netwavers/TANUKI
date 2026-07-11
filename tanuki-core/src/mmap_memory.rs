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

use crate::schema::get_root_as_memory_root;
use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

pub struct MmapMemoryManager {
    // RCU-based active memory data in RAM
    active_data: ArcSwap<Vec<u8>>,
}

impl MmapMemoryManager {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path).context("Failed to open memory file")?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .context("Failed to read memory file")?;
        Ok(Self {
            active_data: ArcSwap::from_pointee(data),
        })
    }

    pub fn update_mapping<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut file = File::open(path).context("Failed to open memory file for update")?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .context("Failed to read memory file for update")?;
        self.active_data.store(Arc::new(data));
        Ok(())
    }

    pub fn search(&self, query_vector: &[f32; 768], top_k: usize) -> Result<Vec<(u64, f32)>> {
        let data_guard = self.active_data.load();
        let mmap_data = &**data_guard;

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
