# tanuki-core: Flat-AST Context Architecture 技術仕様書

`tanuki-core` は、LLM（大規模言語モデル）のコンテキスト窓に入力するプロンプトバッファを、メモリコピー（Allocation）を発生させずに超高速でパッキング、スライシング（プルーニング）、およびメタ構造のシリアライズを行うための Rust 製コアライブラリです。

ローレベルなメモリの効率化（Rust）と、ハイレベルなLLMの認知・指示制御（LLM/DSL）を決定論的に結びつける架け橋として設計されています。

---

## 📌 アーキテクチャ概要

従来のコンテキスト構築（JSONやプレーンテキストによるプロンプト連結）では、以下の課題が存在しました。
1. **RAGでのコンテキスト・スタッフィング**: 大量のノイズ情報が混入することで、LLMが重要指示を見失う（Lost in the Middle）。
2. **アロケーション負荷**: Webサーバーなどでリクエストのたびに文字列の連結・メモリ再確保が発生し、ヒープアロケータやCPUキャッシュが逼迫する。
3. **確率的なコンテキスト削減**: トークン数制限時に重要指示まで適当に削ってしまい、指示の崩壊やセキュリティガードレールの無効化を引き起こす。

`tanuki-core` は、物理バッファ `Vec<u8>` の中にヘッダとペイロードをアライメントしシリアルに詰め込む **`FlatAST`** を導入し、これらの課題を克服します。

さらに、指示の肥大化による全体のトークン予算圧迫に対処するため、重要度（Tier）に基づいて保護ノードの内部構造をインプレースで段階的に縮退（論理削除）させる **「二段フォールバック（縮退トリガー）」** に対応しています。

---

## 💾 1. 物理メモリレイアウト (Physical Layout)

`FlatAST` は、すべてのノードを 16 バイト境界（16-byte Alignment）にパディングした上でシリアルに連結した単一の `Vec<u8>` です。これにより、ヒープアロケーション回数はバッファ拡張時の最小限に抑えられます。

### 1.1 固定長ヘッダ (16 Bytes)
各ノードは、先頭に 16 バイトの固定長メタデータヘッダを持ちます。クロスプラットフォームでの移植性を保証するため、すべてのマルチバイト整数はリトルエンディアン（Le Bytes）で格納されます。

| オフセット (Byte) | フィールド名 | 型 | 説明 |
| :--- | :--- | :--- | :--- |
| `0 - 7` | `node_id` | `u64` | ノード識別子（FNV-1a ハッシュ等で決定論的に算出） |
| `8 - 11` | `payload_len` | `u32` | 後続する可変長ペイロードのバイト長 |
| `12` | `flags` | `u8` | 制御フラグ（Bit 0: `Active` 状態 (1=有効, 0=論理削除), Bit 1: `IsSubNode` 状態 (1=サブノード, 0=通常ノード), Bit 2-3: `node_type` (0: System, 1: Instruction, 2: Reference, 3: History), Bit 4-7: 予約領域） |
| `13` | `priority` | `u8` | 優先度 / 重要度階層（通常のReferenceではプルーニング優先度。SubNodeの場合は `PriorityTier` として解釈） |
| `14 - 15` | `child_count` | `u16` | 直後に連続して配置されているサブノード（子）の数 |

### 1.2 可変長ペイロード (Payload)
ヘッダに続いて、UTF-8 エンコードされた可変長の文字列データが格納されます。
* ペイロードの末尾には、全体の開始位置が 16 バイトの倍数（アライメント境界）になるように `0x00` でパディングが挿入されます。（次のノードの開始位置は `(payload_end + 15) & !15` によりスマートにジャンプ調整されます）

---

## ⚡ 2. インプレース・プルーニングと二段フォールバック（縮退）

トークン予算の上限（`target_token_limit`）を超過した場合、物理的なメモリコピーやバッファの再詰め込みを行わず、ヘッダの `flags` (Active ビット) を書き換えるだけの **高速インプレース論理削除（マーク＆スキップ）** が実行されます。

### 2.1 第一段階（通常プルーニング）
以下の基準で削減対象ノード（`candidates`）が順序付けられます。
1. **絶対保護ノードの除外**: `node_type == 0` (System), `node_type == 1` (Instruction)、および `priority == 0` を持つノードは削減候補から除外され、絶対に保護されます。
2. **優先度降順ソート**: `priority` の値が大きいノード（重要度が低いノード）から順番に削減（Active ビットを `0` に反転）します。
3. **LIFO Eviction**: 優先度が同じである場合は、バッファの後方（インデックスが大きい＝より新しく投入されたコンテキスト）から優先的に削除されます。

### 2.2 第二段階（保護ノード等の内部縮退 / 縮退トリガー）
通常プルーニングを行ってもなお `current_tokens > target_token_limit` である場合（＝保護ノードだけで予算を超えている等）、第二段階へ移行して保護ノード内のサブノード（SubNode）を段階的に縮退させます。
* サブノードの優先度 `priority` フィールドに指定された `PriorityTier` を元に、インプレースで一括論理削除を行います。
  * **`PriorityTier::Tier3` (値: 3)**: 最も削られやすい（補足情報・マニュアルやコード例等）をまず一括論理削除。
  * **`PriorityTier::Tier2` (値: 2)**: 中間の重要度（詳細コンテキスト等）を必要に応じて次に一括論理削除。
  * **`PriorityTier::Tier1` (値: 1)**: 最重要（核心制約・タスク目標）。絶対に削られず死守されます。

### 2.3 予算自動拡張（Auto-Expand）フォールバック
縮退処理を行っても要求予算 `target_token_limit` を超過した場合、エンジンはエラーでクラッシュすることなく、**保護された最小限 of フットプリントまで自動的に予算上限を拡張**して安全に処理を継続させます。

---

## 🔌 3. API インタフェース (FlatAST API)

### 3.1 メソッド仕様

```rust
impl FlatAST {
    /// 空の FlatAST バッファを生成します。
    pub fn new() -> Self;

    /// メモリ上のバッファデータをクリアして再利用可能な状態にします。
    /// （Vec のキャパシティを維持するため、メモリの再アロケーションが防げます）
    pub fn clear(&mut self);

    /// 新しいノードを 16 バイトアライメントでシリアライズしてバッファ末尾に追加します。
    pub fn push_node(
        &mut self,
        node_id: u64,
        node_type: u8,
        priority: u8,
        is_subnode: bool,
        child_count: u16,
        payload: &str,
    );

    /// 特定のノードID（サブノード含む）を見つけ、メモリコピーなしでインプレース論理削除（マーク）します。
    pub fn logical_delete_node(&mut self, target_id: u64) -> bool;

    /// Activeなノードの合計トークン数（バイト長を簡易代用）を取得します。
    pub fn total_tokens(&self) -> u32;

    /// 目標のトークン上限まで優先度に基づいてノードを論理削除し、
    /// 削減完了後の「最終的な総トークン数」を返します（縮退処理や自動拡張時は目標値と異なる値になります）。
    pub fn prune(&mut self, target_token_limit: u32) -> u32;

    /// 有効なノードを結合し、超軽量 DSL プロンプトとしてレンダリングします。
    pub fn render_dsl(&self) -> String;

    /// 有効なノードを人間が読みやすい構造化されたマークダウン文書形式でレンダリングします。
    pub fn render_human_readable(&self) -> String;
}

/// FNV-1a ハッシュを計算して u64 の決定論的ハッシュ値を算出します。
pub fn calculate_fnv1a(s: &str) -> u64;
```

---

## 📝 4. 超軽量カスタム DSL 仕様

`FlatAST::render_dsl` は、各有効ノードから以下の極小トークンプレフィックスを用いて DSL プロンプトを構築します。XMLやJSONのような冗長な「閉じタグ」を排除し、アテンション（Attention）を直後の意味内容に集中させます。また、サブノードはインデント表現されます。

* **`#S: <payload>`** : SystemNode (制約・ペルソナ)
* **`#I: <payload>`** : InstructionNode (目標・タスク指示)
* **`#R[<node_id>,<priority>]: <payload>`** : ReferenceNode (知識ベースへの参照、および優先度)
* **`#H[<node_id>]: <payload>`** : HistoryNode (対話履歴)
* **`  └─ #Sub[<node_id>]: <payload>`** : SubNode (ノード内部に属するサブツリー)

### 4.2 人間用デバッグ文書仕様
`FlatAST::render_human_readable` は、ログやUI上で直感的に読めるようにマークダウン構造化文書形式で出力します。

* **SystemNode** -> `■ SYSTEM CONSTRAINT (システム制約)`
* **InstructionNode** -> `■ ACTIVE GOAL (実行目標)`
* **ReferenceNode** -> `■ REFERENCE KNOWLEDGE [ID: <16桁ハッシュ>, Priority: <優先度>] (参照知識)`
* **HistoryNode** -> `■ CONVERSATION HISTORY [ID: <16桁ハッシュ>] (対話履歴)`
* **SubNode** -> `  └─ ■ SUB-NODE [ID: <16桁ハッシュ>]: <payload>`

---

## ⚠️ 5. 運用監視とメトリクス

`FlatAST::prune` の実行結果、自動拡張フォールバックが発生した場合、呼び出し側は即座にそれを検知して警告を発行したり、 Prometheus 等のカウンタへ違反回数をエクスポートできます。

```rust
let target_limit = 100;
let final_tokens = ast.prune(target_limit);

if final_tokens > target_limit {
    // ⚠️ 自動拡張（予算超過）が発生
    eprintln!(
        "⚠️  [Flat-AST WARNING]: Budget Auto-Expand Triggered! Target limit was {} but expanded to {} due to absolute protected nodes.",
        target_limit,
        final_tokens
    );
}
```

---

## 🔍 6. 検証・実行方法

### 6.1 ユニットテスト
`FlatAST` コアの物理パッキング、アライメント、インプレース・プルーニング、メモリクリア、サブノード論理削除、および二段フォールバックの単体テスト（全 13 ケース）を実行します。

```powershell
cargo test --package tanuki-core
```

### 6.2 Ollama 結合適合性・限界負荷テスト
インメモリ DB およびモック Checkpoint を構築し、6,520 トークン以上のダミー知識ノイズが正しくプルーニングされること、自動拡張フォールバックが働くこと、およびローカル Ollama で正しく指示に沿った推論が行えるかを検証します。

```powershell
# 使用する Ollama モデルを指定（例: gemma4:e2b）
$env:OLLAMA_MODEL="gemma4:e2b"

# テスト実行（限界負荷＆自動拡張検証モード）
cargo run --package tanuki-core --bin test_dsl_inference -- --prune-test
```
