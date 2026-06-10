use tokio::sync::{Mutex, MutexGuard};
use std::sync::Arc;

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
