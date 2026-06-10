use tanuki_core::db::{TanukiDb, KnowledgeNode};
use anyhow::Result;

pub struct SpeculativeEvaluator<'a> {
    db: &'a TanukiDb,
}

#[derive(Debug, Clone)]
pub struct EvaluationResult {
    pub score: f32, // 0.0 to 1.0
    pub connectivity: f32,
    pub blast_radius: f32,
    pub recommendation: String,
}

impl<'a> SpeculativeEvaluator<'a> {
    pub fn new(db: &'a TanukiDb) -> Self {
        Self { db }
    }

    /// 提案された変更（トランザクション実行後）の整合性をスコアリングします。
    pub fn evaluate_proposal(&self) -> Result<EvaluationResult> {
        // 1. Connectivity Scoring (結合密度)
        let connectivity = self.calculate_connectivity()?;
        
        // 2. Blast Radius (影響範囲)
        let blast_radius = self.calculate_blast_radius()?;
        
        // 3. 総評スコア
        let score = (connectivity * 0.7) + ((1.0 - blast_radius) * 0.3);
        
        let recommendation = if score > 0.8 {
            "Strongly aligned with existing architecture. High confidence.".to_string()
        } else if score > 0.5 {
            "Acceptable integration, but may need minor adjustments.".to_string()
        } else {
            "Potential architectural drift detected. Review required.".to_string()
        };

        Ok(EvaluationResult {
            score,
            connectivity,
            blast_radius,
            recommendation,
        })
    }

    fn calculate_connectivity(&self) -> Result<f32> {
        let nodes = self.db.get_all_nodes()?;
        if nodes.is_empty() { return Ok(1.0); }

        // linksテーブルからリンク総数を取得
        let link_count = self.db.get_link_count()?;

        // ノード数に対する平均リンク密度を計算（簡易的な評価式）
        let node_count = nodes.len() as f32;
        let density = (link_count as f32) / node_count;

        // 密度 2.0 (1ノードあたり平均2リンク) を満点 (1.0) としたスコアリング
        let score = (density / 2.0).min(1.0);
        Ok(score)
    }

    fn calculate_blast_radius(&self) -> Result<f32> {
        // 影響が特定のドメイン（source_path）に閉じているか。
        // 広範囲のファイルに及ぶ変更は Blast Radius が高いとみなす。
        Ok(0.1) // Placeholder
    }
}
