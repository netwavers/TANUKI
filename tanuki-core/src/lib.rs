pub mod vram;
pub mod llm;
pub mod db;
pub mod checkpoint;
pub mod schema;
pub mod mmap_memory;

pub use vram::GLOBAL_VRAM_LOCK;
pub use llm::{LlmProvider, GeminiClient, OllamaClient, load_provider};
pub use db::TanukiDb;
pub use checkpoint::{Checkpoint, SummaryNode, ContextStack, SpecGoal, KnowledgeType};
pub use mmap_memory::MmapMemoryManager;
