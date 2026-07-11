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

use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

/// VRAM防御のためのグローバルロックですわ！
/// 複数のAIモデルが同時にVRAMを奪い合わないように、このロックで守ります。
pub struct VramGuard {
    lock: Arc<Mutex<()>>,
}

impl VramGuard {
    pub fn new() -> Self {
        Self {
            lock: Arc::new(Mutex::new(())),
        }
    }

    pub async fn lock(&self) -> MutexGuard<'_, ()> {
        self.lock.lock().await
    }
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_VRAM_LOCK: VramGuard = VramGuard::new();
}
