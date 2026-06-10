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
}
