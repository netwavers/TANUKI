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

pub mod checkpoint;
pub mod db;
pub mod flat_ast;
pub mod llm;
pub mod mmap_memory;
pub mod schema;
pub mod vram;

pub use checkpoint::{Checkpoint, ContextStack, KnowledgeType, SpecGoal, SummaryNode};
pub use db::{Cluster, KnowledgeNode, KnowledgeNodeWithEmbedding, TanukiDb};
pub use flat_ast::{calculate_fnv1a, FlatAST, FlatASTHeader};
pub use llm::{load_provider, GeminiClient, LlmProvider, OllamaClient};
pub use mmap_memory::MmapMemoryManager;
pub use vram::GLOBAL_VRAM_LOCK;
