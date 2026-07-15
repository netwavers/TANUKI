# 🐾 T.A.N.U.K.I. 自動化パイプライン仕様書
### Drive → GitHub → Irminsul 同期アーキテクチャ (Phase 0 実装版)

**作成日**: 2026-07-16  
**作成者**: ナヒーダ（知恵の神 / 思考レイヤー）  
**対象**: ご主人様 & AIエージェント  

---

## 1. 目的

Google Drive 上で編集された仕様書・エージェント指示書を、**決定論的かつ低摩擦**で  
T.A.N.U.K.I. の知識木（Irminsul）に根付かせ、MCP / Resume / すべてのエージェントが即座に参照できるようにする。

## 2. Phase 0: 最小実行可能雛形（実装済み）

### ファイル
- `scripts/drive_to_tanuki.sh` …… Linux版メインスクリプト
- `docs/AUTOMATION_PIPELINE.md` …… 本仕様書

### 使い方 (Linux)

```bash
cd /path/to/TANUKI

# 基本実行
bash scripts/drive_to_tanuki.sh

# オプション
bash scripts/drive_to_tanuki.sh --dry-run
bash scripts/drive_to_tanuki.sh --skip-compile
bash scripts/drive_to_tanuki.sh --skip-factory
```

### 現在の運用フロー（Phase 0）

1. Google Drive フォルダ  
   `T.A.N.U.K.I. Specs & Agent Instructions`  
   https://drive.google.com/drive/folders/1U1681TSGph_OWSl1GeUZDxxb1ioIa55m  
   で仕様書を編集・追加する。

2. 変更した `.md` ファイルをローカルの  
   `documents/specs/`  
   にコピー（またはダウンロード）する。

3. 上記スクリプトを実行する。

4. 完了後、MCP で以下を試す：
   ```
   query_knowledge_ast("AGENT_IRMINSUL_NAV")
   query_knowledge_ast("Whitepaper")
   ```

### スクリプトの内部フロー

```
[1] specs ディレクトリ確認・作成
[2] Git add → commit → push (main)
[3] tanuki-compiler / rebuild_tanuki.py 実行
[4] WAL checkpoint + Factory Sync
[5] 完了ログ出力
```

## 3. 将来 Phase への道筋

| Phase | 内容 | 状態 |
|-------|------|------|
| **0** | 手動 export + bashスクリプト一発実行 | **実装済み** |
| **1** | ポーリング型 DriveWatcher（60秒間隔） | 設計済み |
| **2** | Drive Push Notification + Webhook 完全自動化 | 設計済み |
| **3** | Merkle差分最適化 + 夜間埋め込みバッチ | 構想 |

## 4. 環境変数（.env に追加推奨）

```env
DRIVE_SPECS_FOLDER_ID=1U1681TSGph_OWSl1GeUZDxxb1ioIa55m
TANUKI_SPECS_DIR=documents/specs
```

## 5. 論理的欠陥と注意点（見通す眼）

- **Phase 0 の限界**: Drive からの自動 export はまだ手動。  
  Google Drive API のサービスアカウント or OAuth トークンが必要になるため、Phase 1 で実装する。
- **コンフリクト**: 複数人が同時に Drive を編集した場合、Git push が reject される可能性あり。  
  → 将来は `auto-sync` ブランチ + PR 自動作成に進化させる。
- **コンパイル時間**: 仕様書が増えると embedding が重くなる。  
  → `TANUKI_NO_REDUCE=1` や keep_alive 最適化を併用すること。

## 6. 関連ファイル

- `scripts/sync_kb_to_factory.ps1` …… 既存 Factory 同期（本パイプラインから呼ばれる）
- `docs/AGENT_IRMINSUL_NAV.md` …… エージェント向けナビゲーション
- `docs/KNOWLEDGE_BASE_LIMITATIONS.md` …… 3層同期の弱点

---

*\"知識は、やはり自ら求めてこそ得られるものなのだから。\"*  
— Lesser Lord Kusanali

**Crafted by**: ナヒーダ  
**Supervised by**: かぜまる (ご主人様)
