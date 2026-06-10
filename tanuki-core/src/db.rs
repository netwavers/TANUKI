use rusqlite::{Connection, Result};
use serde::Serialize;
use std::path::Path;

pub struct TanukiDb {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub enum UndoOp {
    DeleteNode(String), // Node ID
    DeleteFileMeta(String), // File Path
    DeleteCluster(String), // Cluster ID
    DeleteLink(String, String, String), // source_id, target_id, link_type
}

pub struct SpeculativeTransaction<'a> {
    db: &'a TanukiDb,
    undo_stack: Vec<UndoOp>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeNode {
    pub id: String,
    pub source_path: String,
    pub file_hash: Option<String>,
    pub context_path: String,
    pub title: String,
    pub content: String,
    pub summary: String,
    pub metadata: String,
}

pub struct FileMeta {
    pub path: String,
    pub mtime: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Cluster {
    pub id: String,
    pub parent_id: String,
    pub title: String,
    pub summary: String,
    pub criteria: String,
}

impl TanukiDb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<()> {
        // 知見の結晶（ノード情報）を保存するテーブル
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS nodes (
                id TEXT PRIMARY KEY,
                source_path TEXT,
                file_hash TEXT,
                context_path TEXT,
                title TEXT,
                content TEXT,
                summary TEXT,
                metadata TEXT,
                embedding BLOB
            )",
            [],
        )?;

        // カラムの追加チェック (マイグレーション)
        {
            let mut stmt = self.conn.prepare("PRAGMA table_info(nodes)")?;
            let columns: Vec<String> = stmt.query_map([], |row| row.get(1))?
                .filter_map(|r| r.ok())
                .collect();
            
            if !columns.contains(&"file_hash".to_string()) {
                println!("  🛠 Migrating database: Adding 'file_hash' column to 'nodes' table...");
                self.conn.execute("ALTER TABLE nodes ADD COLUMN file_hash TEXT", [])?;
            }
        }

        // ファイルメタ情報（差分更新用）を保存するテーブル
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS file_meta (
                path TEXT PRIMARY KEY,
                mtime INTEGER
            )",
            [],
        )?;

        // クラスタリング結果を保存するテーブル
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS clusters (
                id TEXT PRIMARY KEY,
                parent_id TEXT,
                title TEXT,
                summary TEXT,
                navigation_criteria TEXT
            )",
            [],
        )?;

        // ノード間の接続関係（リンク）を保存するテーブル
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS links (
                source_id TEXT,
                target_id TEXT,
                link_type TEXT,
                strength REAL,
                PRIMARY KEY (source_id, target_id, link_type)
            )",
            [],
        )?;

        Ok(())
    }

    pub fn insert_node(&self, id: &str, source_path: &str, file_hash: Option<&str>, context_path: &str, title: &str, content: &str, summary: &str, metadata: &str, embedding: &[f32]) -> Result<()> {
        let embedding_blob = bincode::serialize(embedding).map_err(|e| rusqlite::Error::ToSqlConversionFailure(e))?;
        self.conn.execute(
            "INSERT OR REPLACE INTO nodes (id, source_path, file_hash, context_path, title, content, summary, metadata, embedding) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            (id, source_path, file_hash, context_path, title, content, summary, metadata, embedding_blob),
        )?;
        Ok(())
    }

    pub fn get_all_nodes(&self) -> Result<Vec<KnowledgeNode>> {
        let mut stmt = self.conn.prepare("SELECT id, source_path, file_hash, context_path, title, content, summary, metadata FROM nodes")?;
        let node_iter = stmt.query_map([], |row| {
            Ok(KnowledgeNode {
                id: row.get(0)?,
                source_path: row.get(1)?,
                file_hash: row.get(2)?,
                context_path: row.get(3)?,
                title: row.get(4)?,
                content: row.get(5)?,
                summary: row.get(6)?,
                metadata: row.get(7)?,
            })
        })?;

        let mut nodes = Vec::new();
        for node in node_iter {
            nodes.push(node?);
        }
        Ok(nodes)
    }

    pub fn delete_nodes_by_source(&self, source_path: &str) -> Result<()> {
        self.conn.execute("DELETE FROM nodes WHERE source_path = ?1", [source_path])?;
        Ok(())
    }

    // --- File Meta ---

    pub fn get_file_mtime(&self, path: &str) -> Result<Option<u64>> {
        let mut stmt = self.conn.prepare("SELECT mtime FROM file_meta WHERE path = ?1")?;
        let mut rows = stmt.query([path])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn upsert_file_meta(&self, path: &str, mtime: u64) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO file_meta (path, mtime) VALUES (?1, ?2)",
            (path, mtime),
        )?;
        Ok(())
    }

    pub fn delete_file_meta(&self, path: &str) -> Result<()> {
        self.conn.execute("DELETE FROM file_meta WHERE path = ?1", [path])?;
        Ok(())
    }

    pub fn get_all_file_meta(&self) -> Result<Vec<FileMeta>> {
        let mut stmt = self.conn.prepare("SELECT path, mtime FROM file_meta")?;
        let meta_iter = stmt.query_map([], |row| {
            Ok(FileMeta {
                path: row.get(0)?,
                mtime: row.get(1)?,
            })
        })?;

        let mut metas = Vec::new();
        for meta in meta_iter {
            metas.push(meta?);
        }
        Ok(metas)
    }

    pub fn get_all_clusters(&self) -> Result<Vec<Cluster>> {
        let mut stmt = self.conn.prepare("SELECT id, parent_id, title, summary, navigation_criteria FROM clusters")?;
        let cluster_iter = stmt.query_map([], |row| {
            Ok(Cluster {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                title: row.get(2)?,
                summary: row.get(3)?,
                criteria: row.get(4)?,
            })
        })?;

        let mut clusters = Vec::new();
        for cluster in cluster_iter {
            clusters.push(cluster?);
        }
        Ok(clusters)
    }

    pub fn insert_cluster(&self, id: &str, parent_id: &str, title: &str, summary: &str, criteria: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO clusters (id, parent_id, title, summary, navigation_criteria) VALUES (?1, ?2, ?3, ?4, ?5)",
            (id, parent_id, title, summary, criteria),
        )?;
        Ok(())
    }

    pub fn insert_link(&self, source_id: &str, target_id: &str, link_type: &str, strength: f32) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO links (source_id, target_id, link_type, strength) VALUES (?1, ?2, ?3, ?4)",
            (source_id, target_id, link_type, strength),
        )?;
        Ok(())
    }

    pub fn get_link_count(&self) -> Result<i64> {
        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM links")?;
        stmt.query_row([], |row| row.get(0))
    }

    // --- Speculative Transactions ---

    pub fn start_transaction(&self) -> SpeculativeTransaction<'_> {
        SpeculativeTransaction {
            db: self,
            undo_stack: Vec::new(),
        }
    }
}

impl<'a> SpeculativeTransaction<'a> {
    pub fn insert_node_speculative(&mut self, id: &str, source_path: &str, file_hash: Option<&str>, context_path: &str, title: &str, content: &str, summary: &str, metadata: &str, embedding: &[f32]) -> Result<()> {
        self.db.insert_node(id, source_path, file_hash, context_path, title, content, summary, metadata, embedding)?;
        self.undo_stack.push(UndoOp::DeleteNode(id.to_string()));
        Ok(())
    }

    pub fn insert_cluster_speculative(&mut self, id: &str, parent_id: &str, title: &str, summary: &str, criteria: &str) -> Result<()> {
        self.db.insert_cluster(id, parent_id, title, summary, criteria)?;
        self.undo_stack.push(UndoOp::DeleteCluster(id.to_string()));
        Ok(())
    }

    pub fn insert_link_speculative(&mut self, source_id: &str, target_id: &str, link_type: &str, strength: f32) -> Result<()> {
        self.db.insert_link(source_id, target_id, link_type, strength)?;
        self.undo_stack.push(UndoOp::DeleteLink(source_id.to_string(), target_id.to_string(), link_type.to_string()));
        Ok(())
    }

    pub fn rollback(mut self) -> Result<()> {
        println!("  ⏪ Rolling back speculative transaction ({} operations)...", self.undo_stack.len());
        while let Some(op) = self.undo_stack.pop() {
            match op {
                UndoOp::DeleteNode(id) => {
                    self.db.conn.execute("DELETE FROM nodes WHERE id = ?1", [id])?;
                }
                UndoOp::DeleteFileMeta(path) => {
                    self.db.delete_file_meta(&path)?;
                }
                UndoOp::DeleteCluster(id) => {
                    self.db.conn.execute("DELETE FROM clusters WHERE id = ?1", [id])?;
                }
                UndoOp::DeleteLink(sid, tid, lt) => {
                    self.db.conn.execute("DELETE FROM links WHERE source_id = ?1 AND target_id = ?2 AND link_type = ?3", (sid, tid, lt))?;
                }
            }
        }
        Ok(())
    }

    pub fn commit(mut self) {
        // コミット時はスタックをクリアするだけ（物理的には既に書き込まれているため）
        self.undo_stack.clear();
        println!("  ✅ Speculative transaction committed.");
    }
}
