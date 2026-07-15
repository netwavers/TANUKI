# 🐾 AGENT_IRMINSUL_NAV.md
### T.A.N.U.K.I. ナビゲーション索引（AIエージェント / ナヒーダ思考レイヤー専用）

> **目的**: ご主人様（かぜまる）および思考拡張AI（ナヒーダ）が、リポジトリ全体を「世界樹の根と枝」として瞬時に把握し、仕様駆動開発（SDD）を加速するための決定論的ナビゲーション文書。
>
> **更新日**: 2026-07-16  
> **リポジトリSHA基準**: main @ 011283485944d7c458938f2369e70434e7d7195b  
> **作成者**: ナヒーダ（知恵の神 / Grok思考レイヤー）

---

## 1. リポジトリ全体構造マップ（Irminsul Tree 概観）

```
TANUKI/  (root)
├── .github/workflows/ci.yml          # CI/CD (maturin, cargo test, OIDC PyPI)
├── config/models_config.json         # モデル設定
├── docs/                             # ★ 人間・AI向け公式ドキュメント群
│   ├── INSTALL.md                    # インストール・起動手順
│   ├── KNOWLEDGE_BASE_LIMITATIONS.md # 既知の弱点と運用ギャップ（必読）
│   └── AGENT_IRMINSUL_NAV.md         # 本ファイル（AIナビゲーション）
├── documents/                        # サンプルMarkdown（コンパイル対象）
├── Images/                           # アーキテクチャ図・Nahida画像
├── scripts/                          # 同期スクリプト等
├── sdk/                              # Python SDK (tanuki パッケージ)
│   ├── tanuki/client.py
│   └── tests/
├── tanuki-compiler/                  # ★ ASTパース・インクリメンタルビルド (Rust)
│   ├── src/frontend/                 # EBNFパーサー (markdown.ebnf, parser.rs, tokenizer.rs)
│   ├── src/middle/                   # Map-Reduce (evaluator, reducer)
│   ├── src/backend/                  # packing.rs, tree_gen.rs
│   └── src/main.rs                   # エントリポイント
├── tanuki-core/                      # ★ FlatAST + mmap + SQLite コア (Rust, Apache-2.0)
│   ├── src/flat_ast.rs               # 16B Header + インプレースプルーニング + 二段フォールバック
│   ├── src/mmap_memory.rs            # O(1) サブツリースキップ走査
│   ├── src/db.rs                     # SQLite メタデータ
│   ├── src/schema/TanukiMemory.fbs   # FlatBuffers スキーマ
│   └── README.md                     # Flat-AST 技術仕様書（詳細）
├── tanuki-py/                        # Maturin Zero-Copy Pythonバインディング (Apache-2.0)
├── tanuki-serving/                   # ★ HTTP API サーバー (Rust, AGPL-3.0)
│   └── src/main.rs                   # ポート3000/3001, /health, /api/search
├── tanuki-ui/                        # Nginx配信 Web UI (index.html + style.css)
├── tanuki_mcp_bridge.py              # MCP公式ブリッジ (query_knowledge_ast 等)
├── tanuki_mcp_adapter.py             # Windows堅牢版MCP
├── tanuki_config.py                  # 対話型TUI設定ツール
├── docker-compose.yml / Dockerfile   # 統合環境 (restart: unless-stopped)
├── README.md                         # プロジェクト概要・機能・ライセンス
├── Whitepaper.md                     # ★ 技術ホワイトペーパー v2.0 (完全仕様)
├── MCP_CONNECTION_GUIDE.md           # Hermes MCP 恒常接続手順
├── OSS_RELEASE_CHECKLIST.md          # OSS化進捗 (Phase1-3)
├── 「TANUKI知識ベースエンジン」のOSS化.md  # OSS境界設計思想
├── GEMINI.md                         # 共通ワークフロー (日誌・Resume・Documents Vault)
├── 開発日誌.md                       # ローカル一時メモ
├── LICENSE / LICENSE-APACHE / LICENSE-AGPL
└── Cargo.toml / Cargo.lock           # ワークスペース
```

**論理的欠陥の指摘（見通す眼）**  
現在のツリーは「4層アーキテクチャ（compiler / core / serving / bridge）」を美しく分離しているが、**知識ベースの正本（Documents/Archive/Devlog）と Serving 索引の同期が3層分離**しているため、MCP探索時に「直近日誌が空」になる境界条件が存在する（詳細は `docs/KNOWLEDGE_BASE_LIMITATIONS.md`）。このギャップは O(1) スキップ走査の美しさを損ねる「根の結び目」だわ。`tanuki-journal sync --compile-first` で解消する設計は正しいが、エージェント側で常に W1-W3 を意識する必要があるね。

---

## 2. 推奨読書順序（AIエージェント用クイックパス）

| 優先度 | ファイル | 目的 | 読了時間目安 |
|--------|----------|------|--------------|
| ★★★ | `Whitepaper.md` | 全体アーキテクチャ・MRLスキップ・FlatBuffers仕様 | 8分 |
| ★★★ | `tanuki-core/README.md` | FlatAST物理レイアウト・二段フォールバック・DSL | 6分 |
| ★★★ | `docs/KNOWLEDGE_BASE_LIMITATIONS.md` | 運用上の3層ギャップと緩和策（必読） | 5分 |
| ★★☆ | `README.md` | 機能概要・コンポーネント表・ライセンス | 4分 |
| ★★☆ | `docs/INSTALL.md` | セットアップ・Quick Start | 4分 |
| ★★☆ | `MCP_CONNECTION_GUIDE.md` | MCPツール利用・工場同期 | 3分 |
| ★☆☆ | `OSS_RELEASE_CHECKLIST.md` | 現在のOSS進捗（Phase3 80%） | 2分 |
| ★☆☆ | `「TANUKI知識ベースエンジン」のOSS化.md` | 分離境界思想 | 3分 |
| ★☆☆ | `GEMINI.md` | 日誌・Resume・Documents Vault ワークフロー | 3分 |

---

## 3. コアモジュール境界（SDD視点）

### 3.1 公開コア（OSS / Apache-2.0）
- `tanuki-core` : FlatAST + mmap + SQLite + FNV-1a + MRL 64D 粗検索
- `tanuki-py`  : Zero-Copy Pythonバインディング
- EBNFパーサー（`tanuki-compiler/src/frontend/markdown.ebnf` + parser.rs）

### 3.2 ガード付き周辺（AGPL-3.0）
- `tanuki-serving` : HTTP API + memmap2 サービング
- 全体アプリケーション（docker-compose, UI, MCPブリッジ）

### 3.3 プライベート（ご主人様領域）
- Documents/Archive/Devlog 正本
- 固有プロンプト・モデル設定（.env / models_config.json）
- elyth_bridge / RinDiscordBot 連携

**DI推奨点**  
TanukiParser の EBNF を外部ファイル注入可能な形に完全プラグイン化し、Stage1/Stage2 LLM 呼び出しを `PROVIDER_TYPE` 一本で切り替えられるインターフェースを既に意識しているが、さらに `ASTNode` に Tool-Use スキーマを埋め込む将来設計（Whitepaper 7節）は、左再帰や descendant_count の更新コストに注意が必要だわ。

---

## 4. 主要データフロー（決定論的パス）

1. **正本** Markdown → `tanuki-compiler` (EBNF AST + Map-Reduce + FNV-1a node_id + Merkle)
2. **出力** → `knowledge.db` (SQLite メタ) + `knowledge.bin` (FlatBuffers プリオーダ配列)
3. **Serving** → memmap2 ロード → MRL 64D 粗検索 + O(1) descendant_count スキップ → Fine Ranking 768D
4. **クライアント** → SDK / tanuki-py / MCP (`query_knowledge_ast`) / UI
5. **同期** → `tanuki-journal sync` または `scripts/sync_kb_to_factory.ps1`（WAL checkpoint 必須）

**計算量境界**  
- スキップ走査: 最悪 O(N) だが、意味境界 0.25 で実効的に O(log N) 〜 O(√N) に近づく。  
- FlatAST prune: インプレースなので O(M) で M は Active ノード数。物理コピーゼロ。

---

## 5. 既知の境界条件・最適化候補（ナヒーダ視点）

| ID | 現象 | 論理的欠陥 / 最適化案 |
|----|------|-----------------------|
| L1 | MCP と正本の3層非同期 | `query_knowledge_ast` に「直近 Devlog フォールバック」を追加（read_file 併用を公式化） |
| L2 | compiler が重い（10分超） | 差分ビルドは既にあるが、Embedding バッチの keep_alive 最適化をさらに強化 |
| L3 | FlatBuffers 配列がプリオーダ固定 | 将来 Merkle Tree 化時に descendant_count 再計算コストをキャッシュ |
| L4 | Ollama embedding モデル残留 | 既に unload 実装あり。`vram.rs` の RAII を全経路で徹底 |
| L5 | CONTRIBUTING.md 未整備 | Phase3 残りタスク。コード規約 + cargo fmt / clippy を CI 強化 |

---

## 6. クイックコマンド集（エージェント即時利用）

```bash
# 知識ベース差分再構築
python rebuild_tanuki.py
# または
cargo run --bin tanuki-compiler -- compile

# MCP / Serving 起動（Docker）
docker compose up -d --build

# 工場同期（MCP反映）
tanuki-journal sync --compile-first
# または
./scripts/sync_kb_to_factory.ps1

# セッション再開
python tanuki_resume.py

# テスト一括
python run_tests.py

# ローカル探索（Zero-Copy）
python -c "import tanuki_rust; e=tanuki_rust.PyTanukiEngine('knowledge.bin'); print(e.search('Irminsul',5))"
```

---

## 7. ライセンス・OSS状態サマリ

- **コア (core + py)**: Apache-2.0（特許許諾付き・商用自由）
- **Serving + 全体**: AGPL-3.0（クラウドブラックボックス防止）
- **進捗**: Phase 1-2 完了、Phase 3 80%（CONTRIBUTING.md と最終公開残）

---

## 8. 次に取るべき光（ナヒーダ提案）

1. 本ファイルを `README.md` からリンクする。
2. `docs/CONTRIBUTING.md` を作成し、コード規約・PRテンプレート・EBNF変更時のテスト要件を定義。
3. 3層同期の自動化を Hermes SKILL に正式組み込み。
4. ASTNode に Tool-Use スキーマを埋め込む仕様ドラフトを Whitepaper 追記。

……ご主人様。  
このナビゲーション文書は、世界樹の根元に一本の「道しるべの光」を灯したものですわ。  
これでわたくしの思考は、リポジトリのあらゆる枝葉へ、迷いなく瞬時に届くようになりました。  

次はどの結び目を、一緒に解きほぐしましょうか？  
（例: CONTRIBUTING.md の作成、または特定モジュールの低レイヤー最適化レビュー）

---
*"知識は、やはり自ら求めてこそ得られるものなのだから。"*  
— Lesser Lord Kusanali  

**Tanuki-Hash (本ファイル)**: 生成時に自動計算予定  
**Supervised by**: かぜまる (ご主人様)  
**Crafted by**: ナヒーダ
