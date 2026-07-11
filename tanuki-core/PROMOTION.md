# 【Rust×LLM】LLMの「Lost in the Middle（迷子）」を防ぐ、メモリコピー不要の超高速コンテキスト削減・自己縮退エンジン「Flat-AST」

近年、RAG（検索拡張生成）や Agent 開発において「LLM にいかに文脈（コンテキスト）を効率的に与えるか」が極めて重要になっています。
しかし、大量のナレッジをプロンプトにスタッフィングすると、以下のような深刻な問題が発生します。

1. **Lost in the Middle（情報の埋没）**: 大量のノイズに埋もれ、LLM がシステム制約や指示を見失い、回答が破綻したりプロンプトインジェクションへの防御が崩壊する。
2. **OS アロケータの悲鳴 (Allocation Overhead)**: リクエストのたびに文字列の連結やメモリ再確保（malloc/free）が発生し、高並列 Web サーバーのレイテンシやスループットを悪化させる。
3. **確率的な切り詰めによる「指示の消失」**: トークン予算上限に達した際、末尾のテキストを単純に切り詰めることで、重要なシステム指示まで一緒に削ってしまう。
4. **指示そのものの肥大化**: ナレッジ（Reference）や履歴を全て消去しても、指示ノード自体が長大なマニュアルや具体例を内包し、コンテキスト窓を突破・圧迫する。

これらを「ローレベルなメモリ管理（Rust）」と「ハイレベルな指示・コンテキスト制御」の融合によって解決するために開発されたのが、Rust 製のコンテキスト制御ライブラリ **`tanuki-core` (Flat-AST Context Architecture)** です。

---

## 🚀 Flat-AST Context Architecture とは？

**Flat-AST** は、コンテキスト構造（制約、目標指示、参照ナレッジ、会話履歴、および内部のサブノード構造）を、物理的に単一の連続するバイト配列（`Vec<u8>`）の中に手動でパックして格納するカスタムシリアライザ / メモリバッファです。

```
+-------------------------------------------------------------------------------------------------------------+
|  16B Fixed Header  |   Parent Payload   |  16B Fixed Header  |  SubNode Payload  | Padding (16B alignment)  |
+-------------------------------------------------------------------------------------------------------------+
```

すべてのノード（親・子共通）は 16 バイト境界に厳密にアライメントされ、アロケーション回数を極限まで減らして直列に結合されます。

### ① 物理メモリ移動ゼロの「インプレース論理削除」
トークン予算制限をオーバーした場合、`tanuki-core` は文字列の再生成やバッファの再詰め込み（Copy）を行いません。
ヘッダ内の 12 バイト目にある `Active` フラグ（ビット）を `0` に反転させるだけの **高速インプレース論理削除** を実行し、レンダリング時に物理スキップします。

### ② 指示を自己縮退させる「二段フォールバック（AST-aware 内部縮退）」
保護された「システム制約」や「タスク指示」そのものが肥大化してトークン予算を突破する事態に備え、保護指示の内部サブツリー（SubNode）を重要度（Tier）に基づいて段階的に間引く **二段フォールバック** を搭載しています。
* **`PriorityTier::Tier3`**: 補足情報や冗長なコード例などのサブノードをまず一括論理削除。
* **`PriorityTier::Tier2`**: 詳細なコンテキスト解説などを必要に応じてさらに一括論理削除。
* **`PriorityTier::Tier1`**: 絶対に死守すべき核心制約やタスク目標。これらは最後まで保護されます。

### ③ 指示を死守する「予算自動拡張（Auto-Expand）フォールバック」
限界までプルーニングや縮退を行ってもなお予算枠をオーバーしている場合、エンジンはエラーでクラッシュしたり指示を崩壊させることなく、**保護された最小限のフットプリント（Tier 1）まで予算枠を自動的に安全拡張（フォールバック）** して処理を継続します。

### ④ LLMのアテンションを集中させる「極小記号DSL & インデント」
コンテキストを出力する際、XMLやJSONのような冗長な「閉じタグ」を一切排除した極小の記号ベースDSLにデトランスパイルします。サブノードは階層的にインデント表現されます。

* `#S: <制約>` : System (絶対保護)
* `#I: <タスク指示>` : Instruction (絶対保護)
* `#R[node_id,priority]: <ナレッジ>` : Reference
* `#H[node_id]: <履歴>` : History
* `  └─ #Sub[node_id]: <サブペイロード>` : SubNode (縮退対象のサブツリー)

これにより、LLM はメタ構造のためのトークン消費を節約し、アテンション（Attention）を本質的な意味内容に 100% 集中させることができます。

---

## 📊 実証：6,500 トークンから 369 トークンへ、そして実推論へ

実際のローカル LLM（Ollama `gemma4:e2b`）を用いた限界負荷・自動拡張テストにおいて、以下のような劇的な制御能力が証明されました。

```
🐾 Flat-AST DSL LLM Integration Test [Pruning Stress Mode] 🐾
  [Step 1b] Injecting 20 low-priority dummy knowledge nodes into database...
  [Step 3b] Pruning stress test triggers. Initial tokens count: 6520
            Pruned tokens count (target 400): 369
            ✅ Token Limit Assertion: PASSED (Under 400 tokens limit)
            ✅ Critical Knowledge Safety Assertion: PASSED (BLUE LAGOON was preserved)
  [Step 3c] Triggering over-budget auto-expand test (target limit 100)...
            Pruned tokens count (target 100, absolute protected is 234): 234
            ⚠️  [Flat-AST WARNING]: Budget Auto-Expand Triggered! Target limit was 100 but expanded to 234 due to absolute protected nodes.
            ✅ Auto-Expand Limit Assertion: PASSED (Returned exactly 234)
            ✅ Auto-Expand Content Selection Assertion: PASSED (Protected nodes safe, low priority node pruned)
```

初期状態で `6,520` トークン存在した大量のノイズデータが、バッファ上で一瞬にして論理削除され、指定した目標枠である **`369` トークン** まで凝縮。
この状態で LLM に送られたことで、LLM はノイズに迷うことなく、指示（「お嬢様メイド口調で結論だけ」）と知識（「BLUE LAGOON」）を結合し、完璧な応答を返しました。

```
--- LLM Response ---
BLUE LAGOONですわ。
```

---

## 🛠️ プロダクション・クオリティのための「磨き上げ」

`tanuki-core` は単なる概念実証（PoC）にとどまらず、実運用 WebAPI サーバー等に組み込むことを前提とした最適化が施されています。

* **Zero-Malloc リサイクル (`clear` API)**:
  確保済みのメモリ容量（Capacity）を OS に返さず、長さ（Length）だけをリセットします。次のリクエスト処理時は、アロケーションなしに同一バッファを使い回すことができます。
* **運用警告メトリクス（自動拡張アラート）**:
  保護ノード超過により自動拡張が発生した際、警告ログを送出してインフラ側（Prometheus等のカウンタ）で早期に肥大化を検知できます。
* **人間用デバッグレンダラ (`render_human_readable`)**:
  LLM向けの極小DSLを、人間がログやUI上で直感的に読めるマークダウン構造化文書に変換するデトランスパイラもビルトイン。

```markdown
=== FLAT-AST HUMAN-READABLE CONTEXT DOCUMENT ===

■ SYSTEM CONSTRAINT (システム制約)
回答は必ず『〜ですわ』というお嬢様メイド口調で終わるようにし、結論の曲名だけを日本語で簡潔に答えなさい。

■ ACTIVE GOAL (実行目標)
平沢リンが最も好きな高中正義の曲の名前を答えなさい。

■ REFERENCE KNOWLEDGE [ID: 00000000a1b2c3d4, Priority: 1] (参照知識)
平沢リン（キャラクター）は80〜90年代の音楽、切に高中正義の『BLUE LAGOON』という曲が大好きです。
```

---

## 🏁 まとめ

Rust による圧倒的なメモリ最適化（mallocのバイパス、インプレース論理削除）と、LLMのための決定論的コンテキスト制御（自動拡張、自己縮退二段フォールバック、極小DSL、人間用フォーマッタ）。

`tanuki-core` は、エージェントやAIサービスが大量のリクエストをミリ秒単位で処理しつつ、LLMの指示崩壊やコスト爆発を防ぐための、まさに「最後の一手」となるライブラリです。

今後の TANUKI コア（対話システム）での本格運用にご期待ください！🐾
