use anyhow::Result;
use tanuki_compiler::generate_tree;
use tanuki_core::db::TanukiDb;

fn main() -> Result<()> {
    println!("🐾 T.A.N.U.K.I. Backend Mock Test starting...");
    
    // DB は既存のものを使用
    let db = TanukiDb::open("knowledge.db")?;
    
    // Phase 4: Backend (Generate Tree)
    generate_tree(&db, "output_knowledge")?;
    
    println!("🐾 Backend Mock Test complete!");
    Ok(())
}
