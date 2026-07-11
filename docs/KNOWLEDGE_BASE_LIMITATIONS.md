# TANUKI 知識ベース — 既知の弱点と運用上のギャップ

最終確認: 2026-07-10（TanukiJournal v2 / MCP 日誌取得検証セッション）

この文書は、**開発日誌・resume・MCP 探索**を組み合わせる際にハマりやすい構造上の弱点を記録する。仕様バグの追跡リストではなく、**エージェントとご主人様向けの運用マップ**として扱う。

---

## 1. データが流れる経路（3 層）

| 層 | 場所 | 誰が読むか |
|----|------|------------|
| **正本** | `Documents/Archive/Devlog/YYYY/MM/開発日誌_*.md` | 人間、`read_file`、git |
| **ローカル KB** | `TANUKI/output_knowledge/`（tanuki-compiler 出力） | `tanuki-journal resume`（TanukiExplorer）、ローカル探索 |
| **Serving 索引** | `TANUKI_API_BASE`（例: `http://192.168.2.144:3001`） | **MCP `query_knowledge_ast`**、`/api/search` |

**弱点の核心**: 三層は自動で常に一致しない。`write` 直後は正本だけが最新になりうる。

---

## 2. 弱点一覧

| ID | 弱点 | 影響 | 緩和策 |
|----|------|------|--------|
| W1 | MCP は **ローカル .md を読まない** | 書いた直後の日誌が MCP で空 | 直近は `read_file` / Devlog_Index |
| W2 | **compiler が重い**（10 分超あり） | `write` が日誌だけ先に終わり KB は遅れる | BG 実行・完了後に再検索；`--no-compile` 検討 |
| W3 | **Serving とローカル output の同期**が別工程 | ローカルに載っても MCP（factory）が空のまま | `tanuki-journal sync`（**WAL checkpoint 込み**） |
| W3b | **SQLite WAL** — `knowledge.db` 本体だけ scp すると未マージ | MD5 一致でも factory に新ノードが無い | sync 前 `PRAGMA wal_checkpoint(TRUNCATE)`（`factory_sync.py` 自動） |
| W4 | `query_database` は **スタブ** | SQL で nodes / Devlog を直接引けない | `query_knowledge_ast` のみ前提 |
| W5 | `POST /api/v1/knowledge/search` が **404**（環境依存） | bridge は `GET /api/search` にフォールバック | 新 API 実装 or フォールバックの文書化 |
| W6 | 検索が **Devlog_Index のリンク塊**に偏ることがある | 本文より索引がノイズで返る | クエリに日付・プロジェクト名を含める |
| W7 | **resume（Ollama）と MCP（Serving）でデータ源が違う** | 再開要約と MCP 結果が矛盾しうる | 直近タスクは正本 md + ローカル resume を優先 |
| W8 | 新フォーマット（v2: メタ・検証・決定）の語は **未インデックス時はヒットしない** | 「フォーマットv2」等で空 | compiler 完了 + Serving 同期後に再試行 |

---

## 3. 開発日誌（TanukiJournal）固有

### 3.1 `tanuki-journal write` の副作用

1. Devlog `.md` 作成（即時）
2. `Devlog_Index.md` 更新（即時）
3. `run_tanuki_compiler()`（**非同期・長時間**）

→ **MCP で「今日の日誌」を取る** = 3 が Serving まで反映された後。

### 3.2 確認済み事象（2026-07-10）

- ローカルに存在: `開発日誌_20260710_開発日誌フォーマットv2.md` 等
- `output_knowledge` に `20260710` / `フォーマットv2` **未出現**（compiler 未完了 or 未同期時点）
- `curl .../api/search?q=20260710` → `[]`
- MCP `query_knowledge_ast("開発日誌フォーマットv2")` → 一致なし
- 一方、**過去の索引済み日誌**（例: SonicEther 2026-04-01、rin-discord 関連）は MCP で **段落ノード取得可能**

### 3.3 コンパイル LLM セッション（参考）

- `run_tanuki_compiler` は未設定時 **`TANUKI_MODEL` のみ**注入（keep_alive は既定注入しない）
- compiler 中の Ollama 負荷は write/resume と **並走させない**方がよい

---

## 4. MCP ツール（tanuki-mcp-bridge）の境界

| ツール | 状態 | 備考 |
|--------|------|------|
| `query_knowledge_ast` | 実用可 | Serving 検索 → AST Packed Markdown |
| `query_database` | 未実装 | 「Serving API 拡張待ち」 |
| `update_agent_state` | 受付のみ | 永続化は将来 |

環境変数: `TANUKI_API_BASE`（`mcp_config.json` / bridge 既定と一致させる）

---

## 5. 推奨ワークフロー（エージェント）

| 目的 | 第一選択 | 第二選択 |
|------|----------|----------|
| 直近 1〜3 件の日誌 | `read_file` + `Devlog_Index.md` | — |
| セッション再開（ローカル KB） | `tanuki-journal resume` | — |
| 横断・過去の航跡 | MCP `query_knowledge_ast` | キーワード + 日付 |
| write 後に MCP 確認 | compiler 完了待ち → factory 同期確認 → 再検索 | ダメなら W3 |

**クエリのコツ**: タイトル全文より `SonicEther 20260401`、`TanukiJournal 20260518` のように **日付 + トピック**。

---

## 6. Factory 同期（MCP が読む Serving へ載せる・標準手順）

MCP の `TANUKI_API_BASE` は **tanuki-factory** 上の `tanuki-serving`（例: `http://192.168.2.144:3001`）。  
ローカル（Windows）で compiler を回しても、**次の同期をしない限り MCP は古い索引のまま**。

| 項目 | 値（2026-07-10 時点） |
|------|------------------------|
| SSH ホスト | `tanuki-factory` |
| リモート TANUKI ルート | `/home/tanuki/RinAISystem/TANUKI` |
| ローカル TANUKI ルート | `D:\Projects\PyProjects\TANUKI` |
| 同期対象 | `knowledge.db` + `output_knowledge/` |
| コンテナ | 同ディレクトリで `docker compose restart tanuki-serving` |

### 6.1 前提 — ローカルで compiler 完了

いずれかで **ローカル** `knowledge.db` を更新してから同期する。

```powershell
# 日誌 write 付随（推奨）
tanuki-journal write --title "..." --summary "..." --verify

# またはフル再構築（RAG ポリシー適用）
python D:\Projects\PyProjects\Documents\Archive\Devlog\rebuild_tanuki.py
```

完了確認: `D:\Projects\PyProjects\TANUKI\knowledge.db` の更新時刻、`output_knowledge` に新日誌クラスタが出ること。

### 6.2 一本化コマンド（P0）

```powershell
tanuki-journal sync
# または compiler から一連で:
tanuki-journal sync --compile-first
```

（同等）`D:\Projects\PyProjects\TANUKI\scripts\sync_kb_to_factory.ps1`

処理: `knowledge.db` + `output_knowledge` を tar.gz 化 → `scp` → factory で展開（旧 `knowledge.db` を `.bak` 退避）→ `tanuki-serving` 再起動。

**重要**: compiler は WAL モードのため、同期前に **WAL を main DB にマージ**する（`tanuki-journal sync` / `factory_sync.py` が自動実行）。マージしないとローカルで `sqlite3` 参照時だけ 7/10 日誌が見え、factory は 7/9 のままになる。

### 6.3 同期後の検証

```powershell
curl "http://192.168.2.144:3001/api/search?q=20260710"
# または Hermes MCP: query_knowledge_ast("開発日誌フォーマットv2")
```

空のままなら: ローカル compiler が未完了、または `compile_dirs`（`documents_rag_policy.yaml`）に対象 md が含まれていない可能性を疑う。

### 6.4 備考

- 2026-05-15 頃の `sync_to_server.ps1` / `.bat` はリポジトリ上では見つからない。上記 `sync_kb_to_factory.ps1` を **現行の正** とする。
- factory 上の `knowledge.db` は compose の `./knowledge.db` マウント先。`documents/` はサーバー側に無い構成でも、**Windows でコンパイルした DB を送る**運用で MCP 検索は成立する。

---

## 7. 改善バックログ（優先度の目安）

1. ~~**P0**: compiler 完了後の **Serving 同期**手順を 1 コマンド化~~ → **`TANUKI/scripts/sync_kb_to_factory.ps1`（2026-07-10）**
2. ~~**P1**: `tanuki-journal sync` サブコマンド~~ → **実装済み（2026-07-10）**；`write --no-compile` あり
3. **P1**: cron / タスク完了フローへの `sync --compile-first` の組み込み（Hermes SKILL 明記）
4. **P1**: MCP `read_devlog_recent`（Archive 直読・Serving 非依存）— セキュリティパス制限付き
5. **P2**: `/api/v1/knowledge/search` 実装と bridge の一本化
6. **P2**: `query_database` で `source_path LIKE '%Devlog%'` 参照
7. **P3**: 検索ランキングで Devlog_Index 巨大段落の重み調整

---

## 8. 関連ドキュメント

- `TANUKI/MCP_CONNECTION_GUIDE.md` — MCP 接続手順
- `_agent/skills/TanukiJournal/SKILL.md` — 日誌 v2・`write` / `resume`
- `Documents/Technical/06_General_Specs_System/🛠 MCPネイティブ統合・システム仕様書.md` — 統合仕様

---

*このファイルは検証で得た運用知見を蓄積する。修正で弱点が解消したら該当行を更新し、解消日を括弧で残すこと。*