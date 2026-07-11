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

use crate::db::TanukiDb;
use crate::flat_ast::{calculate_fnv1a, FlatAST};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Checkpoint {
    pub session_id: String,
    pub timestamp: String,
    pub ast_root_hash: String,
    pub context_stack: ContextStack,
    pub knowledge_graph_summary: KnowledgeGraphSummary,
    pub strategist_notes: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContextStack {
    pub current_scope: String,
    pub active_constraints: Vec<String>,
    pub spec_driven_goals: Vec<SpecGoal>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpecGoal {
    pub id: String,
    pub status: String,
    pub priority: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KnowledgeGraphSummary {
    pub nodes: Vec<SummaryNode>,
    pub hot_zones: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KnowledgeType {
    Node,
    Link,
    Constraint,
    Spec,
    Devlog,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SummaryNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: KnowledgeType,
    pub symbol: String,
    pub link: String,
}

impl Checkpoint {
    pub fn new(session_id: String, ast_root_hash: String) -> Self {
        use chrono::Utc;
        Self {
            session_id,
            timestamp: Utc::now().to_rfc3339(),
            ast_root_hash,
            context_stack: ContextStack {
                current_scope: "root".to_string(),
                active_constraints: Vec::new(),
                spec_driven_goals: Vec::new(),
            },
            knowledge_graph_summary: KnowledgeGraphSummary {
                nodes: Vec::new(),
                hot_zones: Vec::new(),
            },
            strategist_notes: "Initialized new session.".to_string(),
        }
    }

    /// Checkpoint と DB からデータを集約し、FlatAST にトランスパイルします。
    pub fn to_flat_ast(&self, db: &TanukiDb) -> Result<FlatAST, rusqlite::Error> {
        let mut ast = FlatAST::new();

        // 1. 制約 -> SystemNode (Type=0, Priority=0 [保護])
        for constraint in &self.context_stack.active_constraints {
            let node_id = calculate_fnv1a(constraint);
            ast.push_node(node_id, 0, 0, false, 0, constraint);
        }

        // 2. アクティブな目標 -> InstructionNode (Type=1, Priority=0 [保護])
        for goal in &self.context_stack.spec_driven_goals {
            if goal.status == "PENDING" {
                let node_id = calculate_fnv1a(&goal.id);
                ast.push_node(node_id, 1, 0, false, 0, &goal.id);
            }
        }

        // 3. ナレッジグラフサマリー内のノード -> ReferenceNode (Type=2)
        for node in &self.knowledge_graph_summary.nodes {
            // Link や関係性はスキップし、実体ノードのみ処理
            if node.node_type == KnowledgeType::Link {
                continue;
            }

            // DBから知見ノードのコンテンツを取得
            if let Some(kb_node) = db.get_node(&node.id)? {
                // 16進数ハッシュ文字列 (例: "f4a45524") を u64 に変換して node_id とする
                let node_id =
                    u64::from_str_radix(&node.id, 16).unwrap_or_else(|_| calculate_fnv1a(&node.id));

                // 優先度の動的決定ロジック
                let mut priority = 5u8; // デフォルトの優先度
                if self.knowledge_graph_summary.hot_zones.contains(&node.id) {
                    priority = 1; // 重要ノードは最優先
                } else {
                    // 目標に関連するノードの場合は、目標の優先度を引き継ぐ
                    for goal in &self.context_stack.spec_driven_goals {
                        if goal.id.contains(&node.id) || node.id.contains(&goal.id) {
                            priority = goal.priority.max(1).min(255) as u8;
                            break;
                        }
                    }
                }

                ast.push_node(node_id, 2, priority, false, 0, &kb_node.content);
            }
        }

        Ok(ast)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::TanukiDb;

    #[test]
    fn test_to_flat_ast() -> Result<(), Box<dyn std::error::Error>> {
        // 1. インメモリDBの作成とデータ投入
        let db = TanukiDb::open(":memory:")?;
        db.insert_node(
            "f4a45524",
            "src/test.rs",
            None,
            "root",
            "Title",
            "Important algorithm content",
            "Summary",
            "{}",
            &[],
        )?;

        // 2. Checkpointの構築
        let mut checkpoint = Checkpoint::new("test-session".to_string(), "hash123".to_string());
        checkpoint
            .context_stack
            .active_constraints
            .push("Constraint 1".to_string());

        checkpoint.context_stack.spec_driven_goals.push(SpecGoal {
            id: "f4a45524_goal".to_string(),
            status: "PENDING".to_string(),
            priority: 3,
        });

        checkpoint.knowledge_graph_summary.nodes.push(SummaryNode {
            id: "f4a45524".to_string(),
            node_type: KnowledgeType::Node,
            symbol: "MySymbol".to_string(),
            link: "MyLink".to_string(),
        });

        // 3. トランスパイル実行
        let ast = checkpoint.to_flat_ast(&db)?;

        // 4. DSLレンダリングと検証
        let dsl = ast.render_dsl();

        // System node, Instruction node, Reference node (ID: 0xf4a45524 = 4104410404, Priority: 3)
        assert!(dsl.contains("#S: Constraint 1"));
        assert!(dsl.contains("#I: f4a45524_goal"));
        assert!(dsl.contains("#R[4104410404,3]: Important algorithm content"));

        // 5. 優先度の動的決定検証（hot_zones にある場合は優先度 1）
        checkpoint
            .knowledge_graph_summary
            .hot_zones
            .push("f4a45524".to_string());
        let ast_hot = checkpoint.to_flat_ast(&db)?;
        let dsl_hot = ast_hot.render_dsl();
        assert!(dsl_hot.contains("#R[4104410404,1]: Important algorithm content"));

        Ok(())
    }
}
