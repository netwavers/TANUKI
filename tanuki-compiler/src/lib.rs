pub mod frontend;
pub mod middle;
pub mod backend;

pub use frontend::{parse_markdown_file, MdNode, MdNodeKind};
pub use middle::{process_nodes, reduce_knowledge, evaluator::SpeculativeEvaluator};
pub use backend::{generate_tree, generate_checkpoint, calculate_ast_root_hash};
