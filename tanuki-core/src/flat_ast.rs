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

use std::str;

/// サブノードの優先度（重要度）階層
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PriorityTier {
    Tier3 = 3, // 最も削られやすい（補足・コード例）
    Tier2 = 2, // 中間の重要度（詳細コンテキスト）
    Tier1 = 1, // 最重要（核心制約・タスク目標。絶対に削らない）
}

/// ノードのヘッダ構造体（16バイト固定サイズ、16バイトアライメント）
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlatASTHeader {
    pub node_id: u64,     // 8B: FNV-1a ハッシュ等で決定論的に算出
    pub payload_len: u32, // 4B: このノードの純粋なテキスト(UTF-8)データのバイト長
    pub flags: u8, // 1B: ビットマスク (Bit 0: Active, Bit 1: IsSubNode, Bit 2-3: node_type, Bit 4-7: Reserved)
    pub priority: u8, // 1B: 優先度スコア (0-255)
    pub child_count: u16, // 2B: 直後に連続して配置されているサブノード（子）の数
}

impl FlatASTHeader {
    /// ノードが有効かどうかを判定します。
    pub fn is_active(&self) -> bool {
        (self.flags & 0x01) != 0
    }

    /// サブノードかどうかを判定します。
    pub fn is_subnode(&self) -> bool {
        (self.flags & 0x02) != 0
    }

    /// ノードのタイプ（0: System, 1: Instruction, 2: Reference, 3: History）を取得します。
    pub fn node_type(&self) -> u8 {
        (self.flags >> 2) & 0x03
    }
}

/// FNV-1a ハッシュを計算して u64 のノードIDを算出します。
pub fn calculate_fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// メモリ上に連続してパックされた AST 構造体
#[derive(Debug, Clone, Default)]
pub struct FlatAST {
    data: Vec<u8>,
}

impl FlatAST {
    /// 新しい FlatAST を生成します。
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// バッファが空かどうかを返します。
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// メモリ容量（Capacity）を維持したまま、バッファデータをクリアして再利用可能な状態にします。
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// ノードをパックしてバッファの末尾に追加します。
    /// 各ノードは16バイトアライメント（開始オフセットが16の倍数）になるようパディングされます。
    pub fn push_node(
        &mut self,
        node_id: u64,
        node_type: u8,
        priority: u8,
        is_subnode: bool,
        child_count: u16,
        payload: &str,
    ) {
        let payload_bytes = payload.as_bytes();
        let payload_len = payload_bytes.len() as u32;

        let mut flags = 0u8;
        flags |= 0x01; // Active = 1
        if is_subnode {
            flags |= 0x02; // IsSubNode = 1
        }
        flags |= (node_type & 0x03) << 2; // node_type

        // ヘッダ（16バイト）を手動でシリアライズ
        let mut header_bytes = [0u8; 16];
        header_bytes[0..8].copy_from_slice(&node_id.to_le_bytes());
        header_bytes[8..12].copy_from_slice(&payload_len.to_le_bytes());
        header_bytes[12] = flags;
        header_bytes[13] = priority;
        header_bytes[14..16].copy_from_slice(&child_count.to_le_bytes());

        self.data.extend_from_slice(&header_bytes);

        // ペイロードを追加
        self.data.extend_from_slice(payload_bytes);

        // 16バイトアライメントのためのパディングを追加
        // 全体のサイズ (16 + payload_len) を 16バイトの倍数にする
        let pad_len = (16 - (payload_len % 16)) % 16;
        if pad_len > 0 {
            self.data
                .extend(std::iter::repeat(0).take(pad_len as usize));
        }
    }

    /// 各ノードのヘッダとペイロードへの参照を走査する内部ヘルパー
    fn iter_nodes(&self) -> NodeIter<'_> {
        NodeIter {
            data: &self.data,
            offset: 0,
        }
    }

    /// Activeなノードの合計トークン数（新仕様では payload_len を代用）を取得します。
    pub fn total_tokens(&self) -> u32 {
        let mut total = 0;
        for (header, _, _) in self.iter_nodes() {
            if header.is_active() {
                total += header.payload_len;
            }
        }
        total
    }

    /// 目標トークン数を下回るまで、優先度の低い（priority値が大きい）ノードから順に無効化（論理削除）します。
    /// ただし、System(0) や Instruction(1) ノード、および priority = 0 のノードは絶対に保護されます。
    /// 削減完了後の「最終的な総トークン数」を返します。
    pub fn prune(&mut self, target_token_limit: u32) -> u32 {
        let mut current_tokens = self.total_tokens();
        if current_tokens <= target_token_limit {
            return current_tokens; // 既に上限以下の場合は何もしない
        }

        // --- 第一段階（通常プルーニング） ---
        // 削減候補ノード（Dynamic かつ priority > 0 且つ Active なもの、かつ通常ノードであるもの）を収集
        let mut candidates = Vec::new();
        for (header, _, offset) in self.iter_nodes() {
            let is_active = header.is_active();
            let node_type = header.node_type();
            let is_subnode = header.is_subnode();
            let is_critical =
                node_type == 0 || node_type == 1 || header.priority == 0 || is_subnode;

            if is_active && !is_critical {
                candidates.push(PruneCandidate {
                    offset,
                    priority: header.priority,
                    token_count: header.payload_len,
                });
            }
        }

        // 削減優先順位でソート
        // 1. 優先度（priority）の高い値（＝重要度が低い）から順に削除
        // 2. 優先度が同じ場合は、バッファの後方（インデックスが大きい＝より新しいデータ、または後ろのReference）から順に削除
        candidates.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority) // 降順 (大きな値が先)
                .then_with(|| b.offset.cmp(&a.offset)) // 同値ならオフセットが大きい（後方）ものを先
        });

        // 目標を下回るまで順次無効化
        for candidate in candidates {
            if current_tokens <= target_token_limit {
                break;
            }
            // dataバッファ内の flags (オフセット + 12 バイト目) の Active ビット (Bit0) をクリア
            self.data[candidate.offset + 12] &= !1;
            current_tokens -= candidate.token_count;
        }

        current_tokens = self.total_tokens();

        // --- 第二段階（保護ノード等の内部縮退 / 縮退トリガー） ---
        if current_tokens > target_token_limit {
            // まずは最も低い Tier3 のサブノードをインプレースで一括論理削除
            self.deactivate_subnodes_by_tier(PriorityTier::Tier3);
            current_tokens = self.total_tokens();

            // それでも超えていれば Tier2 も落とす
            if current_tokens > target_token_limit {
                self.deactivate_subnodes_by_tier(PriorityTier::Tier2);
                current_tokens = self.total_tokens();
            }
        }

        current_tokens
    }

    /// 指定した重要度階層（Tier）のサブノードの Active ビットを一括で 0 にします。
    fn deactivate_subnodes_by_tier(&mut self, tier: PriorityTier) {
        let mut cursor = 0;
        let buffer_len = self.data.len();

        while cursor < buffer_len {
            if cursor + 16 > buffer_len {
                break;
            }

            let flags = self.data[cursor + 12];
            let is_subnode = (flags & 0x02) != 0;
            let priority = self.data[cursor + 13];

            if is_subnode && priority == (tier as u8) {
                self.data[cursor + 12] &= !0x01; // 論理削除！
            }

            let payload_len =
                u32::from_le_bytes(self.data[cursor + 8..cursor + 12].try_into().unwrap());
            let payload_end = cursor + 16 + payload_len as usize;
            // 次のノードの開始位置（16Bアライメント調整）へカーソルをジャンプ
            cursor = (payload_end + 15) & !15;
        }
    }

    /// 有効な（Activeな）ノードのペイロードを超軽量DSLのルールに基づいて結合した文字列を作成します。
    pub fn render_dsl(&self) -> String {
        let mut result = String::new();

        for (header, payload, _) in self.iter_nodes() {
            if !header.is_active() {
                continue; // 無効なノードはスキップ
            }

            if header.is_subnode() {
                // サブノード（コード断片やセクション内部）の出力
                result.push_str(&format!("  └─ #Sub[{}]: {}\n", header.node_id, payload));
            } else {
                match header.node_type() {
                    0 => {
                        // System: #S: <payload>\n
                        result.push_str(&format!("#S: {}\n", payload));
                    }
                    1 => {
                        // Instruction: #I: <payload>\n
                        result.push_str(&format!("#I: {}\n", payload));
                    }
                    2 => {
                        // Reference: #R[<node_id>,<priority>]: <payload>\n
                        result.push_str(&format!(
                            "#R[{},{}]: {}\n",
                            header.node_id, header.priority, payload
                        ));
                    }
                    3 => {
                        // History: #H[<node_id>]: <payload>\n
                        result.push_str(&format!("#H[{}]: {}\n", header.node_id, payload));
                    }
                    _ => {
                        // 未定義タイプはそのまま出力
                        result.push_str(&format!("#?: {}\n", payload));
                    }
                }
            }
        }

        result
    }

    /// 有効な（Activeな）ノードを人間が読みやすいマークダウン文書形式でレンダリングします。
    pub fn render_human_readable(&self) -> String {
        let mut result = String::new();
        result.push_str("=== FLAT-AST HUMAN-READABLE CONTEXT DOCUMENT ===\n\n");

        for (header, payload, _) in self.iter_nodes() {
            if !header.is_active() {
                continue; // 無効なノードはスキップ
            }

            if header.is_subnode() {
                result.push_str(&format!(
                    "  └─ ■ SUB-NODE [ID: {:016x}]: {}\n\n",
                    header.node_id, payload
                ));
            } else {
                match header.node_type() {
                    0 => {
                        result.push_str(&format!(
                            "■ SYSTEM CONSTRAINT (システム制約)\n{}\n\n",
                            payload
                        ));
                    }
                    1 => {
                        result.push_str(&format!("■ ACTIVE GOAL (実行目標)\n{}\n\n", payload));
                    }
                    2 => {
                        result.push_str(&format!(
                            "■ REFERENCE KNOWLEDGE [ID: {:016x}, Priority: {}] (参照知識)\n{}\n\n",
                            header.node_id, header.priority, payload
                        ));
                    }
                    3 => {
                        result.push_str(&format!(
                            "■ CONVERSATION HISTORY [ID: {:016x}] (対話履歴)\n{}\n\n",
                            header.node_id, payload
                        ));
                    }
                    _ => {
                        result.push_str(&format!(
                            "■ UNKNOWN NODE [Type: {}]\n{}\n\n",
                            header.node_type(),
                            payload
                        ));
                    }
                }
            }
        }

        result
    }

    /// 特定のノードID（サブノード含む）を見つけ、物理コピーなしでインプレース論理削除（マーク）します。
    pub fn logical_delete_node(&mut self, target_id: u64) -> bool {
        let mut cursor = 0;
        let buffer_len = self.data.len();

        while cursor < buffer_len {
            if cursor + 16 > buffer_len {
                break;
            }

            let node_id = u64::from_le_bytes(self.data[cursor..cursor + 8].try_into().unwrap());

            if node_id == target_id {
                // Bit 0 を反転（Activeフラグを落とす）
                self.data[cursor + 12] &= !0x01;
                return true;
            }

            let payload_len =
                u32::from_le_bytes(self.data[cursor + 8..cursor + 12].try_into().unwrap());

            let payload_end = cursor + 16 + payload_len as usize;
            // 次のノードの開始位置（16Bアライメント調整）へカーソルをジャンプ
            cursor = (payload_end + 15) & !15;
        }
        false
    }
}

/// プルーニング評価用の候補構造体
struct PruneCandidate {
    offset: usize,
    priority: u8,
    token_count: u32,
}

/// FlatAST のノードを走査するための内部イテレータ
struct NodeIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = (FlatASTHeader, &'a str, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 16 > self.data.len() {
            return None;
        }

        // バイト列から安全にデシリアライズ
        let node_id =
            u64::from_le_bytes(self.data[self.offset..self.offset + 8].try_into().unwrap());
        let payload_len = u32::from_le_bytes(
            self.data[self.offset + 8..self.offset + 12]
                .try_into()
                .unwrap(),
        );
        let flags = self.data[self.offset + 12];
        let priority = self.data[self.offset + 13];
        let child_count = u16::from_le_bytes(
            self.data[self.offset + 14..self.offset + 16]
                .try_into()
                .unwrap(),
        );

        let header = FlatASTHeader {
            node_id,
            payload_len,
            flags,
            priority,
            child_count,
        };

        let payload_start = self.offset + 16;
        let payload_end = payload_start + payload_len as usize;

        if payload_end > self.data.len() {
            return None; // バッファオーバーラン防止
        }

        let payload_bytes = &self.data[payload_start..payload_end];
        let payload = str::from_utf8(payload_bytes).unwrap_or("");

        let current_offset = self.offset;

        // 次のノードのオフセットへ進む (16バイトアライメント調整)
        self.offset = (payload_end + 15) & !15;

        Some((header, payload, current_offset))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_alignment() {
        let mut ast = FlatAST::new();
        ast.push_node(100, 0, 0, false, 0, "hello");
        ast.push_node(200, 2, 5, false, 0, "world!!!");

        let nodes: Vec<_> = ast.iter_nodes().collect();
        assert_eq!(nodes.len(), 2);

        // System node
        assert_eq!(nodes[0].0.node_id, 100);
        assert_eq!(nodes[0].0.node_type(), 0);
        assert_eq!(nodes[0].0.priority, 0);
        assert_eq!(nodes[0].1, "hello");
        assert_eq!(nodes[0].2, 0); // offset = 0

        // Reference node
        assert_eq!(nodes[1].0.node_id, 200);
        assert_eq!(nodes[1].0.node_type(), 2);
        assert_eq!(nodes[1].0.priority, 5);
        assert_eq!(nodes[1].1, "world!!!");
        // "hello" の長さは 5 bytes.
        // 16 + 5 = 21. 16アライメントのために +11 パディング -> 32.
        assert_eq!(nodes[1].2, 32); // offset = 32
    }

    #[test]
    fn test_total_tokens() {
        let mut ast = FlatAST::new();
        ast.push_node(1, 0, 0, false, 0, "system"); // len = 6
        ast.push_node(2, 2, 5, false, 0, "ref"); // len = 3
        assert_eq!(ast.total_tokens(), 9);
    }

    #[test]
    fn test_pruning() {
        let mut ast = FlatAST::new();
        // System node: protected (node_type = 0)
        ast.push_node(1, 0, 0, false, 0, "system_msg"); // len = 10
                                                        // Critical custom node: protected (priority = 0)
        ast.push_node(2, 2, 0, false, 0, "critical_reference_info"); // len = 23
                                                                     // Low priority dynamic node: priority = 10
        ast.push_node(3, 2, 10, false, 0, "low_priority_ref_info"); // len = 21
                                                                    // Medium priority dynamic node: priority = 5
        ast.push_node(4, 2, 5, false, 0, "med_priority_reference_info"); // len = 27

        let initial_tokens = ast.total_tokens();
        assert_eq!(initial_tokens, 10 + 23 + 21 + 27); // 81

        // Target: 60.
        // 低優先度 "low_priority_ref_info" (21) が最初に消えて、81 - 21 = 60 になる。
        // 次に "med_priority_reference_info" (27) が消えて、60 - 27 = 33 になる。
        // "system_msg" と "critical_reference_info" は保護されるため、33 で止まるはず。
        let pruned = ast.prune(60);
        assert_eq!(pruned, 60);
        assert_eq!(ast.total_tokens(), 60);

        let dsl = ast.render_dsl();
        assert!(dsl.contains("#S: system_msg"));
        assert!(dsl.contains("#R[2,0]: critical_reference_info"));
        assert!(!dsl.contains("low_priority_ref_info"));
        assert!(dsl.contains("med_priority_reference_info"));
    }

    #[test]
    fn test_prune_same_priority() {
        let mut ast = FlatAST::new();
        ast.push_node(1, 0, 0, false, 0, "system_msg"); // len = 10
                                                        // 同一優先度 (priority = 5) の動的ノードが2つ
        ast.push_node(2, 2, 5, false, 0, "first_reference_info"); // len = 20
        ast.push_node(3, 2, 5, false, 0, "second_reference_info"); // len = 21

        assert_eq!(ast.total_tokens(), 51);

        // Target: 40.
        // 同一優先度のうち、バッファ後方にある "second_reference_info" が先に消えて、51 - 21 = 30 になるはず。
        let pruned = ast.prune(40);
        assert_eq!(pruned, 30);
        assert_eq!(ast.total_tokens(), 30);

        let dsl = ast.render_dsl();
        assert!(dsl.contains("first_reference_info"));
        assert!(!dsl.contains("second_reference_info"));
    }

    #[test]
    fn test_prune_auto_expand() {
        let mut ast = FlatAST::new();
        // System node: protected (len = 20)
        ast.push_node(1, 0, 0, false, 0, "system_msg_protected");
        // Dynamic node: low priority (len = 16, priority = 5)
        ast.push_node(2, 2, 5, false, 0, "dynamic_node_ref");

        assert_eq!(ast.total_tokens(), 36);

        // Target: 10 (SystemNodeのサイズ20を下回る)
        // dynamic_node_ref (16) は消去されるが、system_msg_protected (20) は保護され、最終トークンは 20 に自動拡張される
        let pruned = ast.prune(10);
        assert_eq!(pruned, 20);
        assert_eq!(ast.total_tokens(), 20);

        let dsl = ast.render_dsl();
        assert!(dsl.contains("system_msg_protected"));
        assert!(!dsl.contains("dynamic_node_ref"));
    }

    #[test]
    fn test_render_dsl() {
        let mut ast = FlatAST::new();
        ast.push_node(10, 0, 0, false, 0, "SysPrompt");
        ast.push_node(20, 1, 0, false, 0, "Do this");
        ast.push_node(30, 2, 3, false, 0, "Data context");
        ast.push_node(40, 3, 5, false, 0, "Hello user");

        let dsl = ast.render_dsl();
        let expected = "#S: SysPrompt\n#I: Do this\n#R[30,3]: Data context\n#H[40]: Hello user\n";
        assert_eq!(dsl, expected);
    }

    #[test]
    fn test_clear() {
        let mut ast = FlatAST::new();
        ast.push_node(1, 0, 0, false, 0, "first");
        assert_eq!(ast.total_tokens(), 5);
        assert!(!ast.is_empty());

        // バッファをクリア
        ast.clear();
        assert!(ast.is_empty());
        assert_eq!(ast.total_tokens(), 0);

        // クリア後、再度ノードを追加して正常に機能することを確認
        ast.push_node(2, 1, 0, false, 0, "second");
        assert_eq!(ast.total_tokens(), 6);
        let dsl = ast.render_dsl();
        assert_eq!(dsl, "#I: second\n");
    }

    #[test]
    fn test_render_human_readable() {
        let mut ast = FlatAST::new();
        ast.push_node(1, 0, 0, false, 0, "System rule");
        ast.push_node(2, 1, 0, false, 0, "Run test");
        ast.push_node(3, 2, 3, false, 0, "Fact data");
        ast.push_node(4, 3, 0, false, 0, "History message");

        let doc = ast.render_human_readable();

        assert!(doc.contains("=== FLAT-AST HUMAN-READABLE CONTEXT DOCUMENT ==="));
        assert!(doc.contains("■ SYSTEM CONSTRAINT (システム制約)\nSystem rule"));
        assert!(doc.contains("■ ACTIVE GOAL (実行目標)\nRun test"));
        assert!(doc.contains(
            "■ REFERENCE KNOWLEDGE [ID: 0000000000000003, Priority: 3] (参照知識)\nFact data"
        ));
        assert!(doc
            .contains("■ CONVERSATION HISTORY [ID: 0000000000000004] (対話履歴)\nHistory message"));
    }

    #[test]
    fn test_logical_delete_and_subnode() {
        let mut ast = FlatAST::new();
        // 親ノードの登録 (child_count = 2)
        ast.push_node(100, 2, 5, false, 2, "parent_payload");
        // サブノードの登録
        ast.push_node(101, 2, 5, true, 0, "sub_node_1");
        ast.push_node(102, 2, 5, true, 0, "sub_node_2");

        let dsl_before = ast.render_dsl();
        assert!(dsl_before.contains("#R[100,5]: parent_payload"));
        assert!(dsl_before.contains("  └─ #Sub[101]: sub_node_1"));
        assert!(dsl_before.contains("  └─ #Sub[102]: sub_node_2"));

        // sub_node_2 (102) をインプレース論理削除
        let success = ast.logical_delete_node(102);
        assert!(success);

        let dsl_after = ast.render_dsl();
        assert!(dsl_after.contains("#R[100,5]: parent_payload"));
        assert!(dsl_after.contains("  └─ #Sub[101]: sub_node_1"));
        assert!(!dsl_after.contains("sub_node_2")); // 論理削除されたので非Active

        // 存在しないノードの削除試行
        let fail = ast.logical_delete_node(999);
        assert!(!fail);
    }

    #[test]
    fn test_prune_two_stage_fallback() {
        let mut ast = FlatAST::new();
        // 親ノード (System): node_type = 0, priority = 0 (保護), child_count = 3
        ast.push_node(1, 0, 0, false, 3, "system_msg_protected"); // len = 20
                                                                  // サブノードの登録
        ast.push_node(2, 0, 3, true, 0, "sub_node_t3"); // len = 11, priority = Tier3
        ast.push_node(3, 0, 2, true, 0, "sub_node_t2_middle"); // len = 18, priority = Tier2
        ast.push_node(4, 0, 1, true, 0, "sub_node_t1_critical"); // len = 20, priority = Tier1
                                                                 // 通常Referenceノード
        ast.push_node(5, 2, 5, false, 0, "dummy_reference_node"); // len = 20, priority = 5 (削減可能)

        // 合計トークン数 = 20 + 11 + 18 + 20 + 20 = 89
        assert_eq!(ast.total_tokens(), 89);

        // 1. target_limit = 80 の場合：
        // 通常Reference (20) が削減され、89 - 20 = 69 に。
        // 69 <= 80 なので、二段フォールバック（縮退トリガー）は発火しない。
        let pruned_1 = ast.prune(80);
        assert_eq!(pruned_1, 69);
        let dsl_1 = ast.render_dsl();
        assert!(dsl_1.contains("sub_node_t3"));
        assert!(dsl_1.contains("sub_node_t2_middle"));
        assert!(dsl_1.contains("sub_node_t1_critical"));
        assert!(!dsl_1.contains("dummy_reference_node"));

        // 2. target_limit = 65 の場合：
        // 通常Reference (20) が削減された段階で 69 > 65 なので縮退トリガー発火。
        // まず Tier3 (11) が消えて 69 - 11 = 58 になる。
        // 58 <= 65 なので終了。Tier2 と Tier1 は残る。
        let mut ast_2 = ast.clone();
        let pruned_2 = ast_2.prune(65);
        assert_eq!(pruned_2, 58);
        let dsl_2 = ast_2.render_dsl();
        assert!(!dsl_2.contains("sub_node_t3")); // 消去
        assert!(dsl_2.contains("sub_node_t2_middle")); // 残存
        assert!(dsl_2.contains("sub_node_t1_critical")); // 残存
        assert!(!dsl_2.contains("dummy_reference_node")); // 消去

        // 3. target_limit = 43 の場合：
        // 通常Reference (20) が消去 (69)。
        // Tier3 (11) が消去 (58)。まだ 58 > 43 なので、Tier2 (18) も消去。
        // 58 - 18 = 40。40 <= 43 なので終了。
        // 最終トークン数は 40 (system_msg_protected: 20 + sub_node_t1_critical: 20) になる。
        let mut ast_3 = ast.clone();
        let pruned_3 = ast_3.prune(43);
        assert_eq!(pruned_3, 40);
        let dsl_3 = ast_3.render_dsl();
        assert!(!dsl_3.contains("sub_node_t3")); // 消去
        assert!(!dsl_3.contains("sub_node_t2_middle")); // 消去
        assert!(dsl_3.contains("sub_node_t1_critical")); // 残存
    }
}
