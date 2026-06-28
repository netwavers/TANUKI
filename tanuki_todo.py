#!/usr/bin/env python3
import os
import sys
import re
import subprocess

# ==============================================================================
# 🐾 T.A.N.U.K.I. 汎用対話型進捗・チェックリスト管理ツール (tanuki_todo.py)
# ==============================================================================
#
# このツールはMarkdownファイル中のチェックボックス（- [ ], - [/], - [x]）を
# 抽出し、ターミナル上で対話的にステータスを切り替え・保存する汎用スクリプトですわ🐾
#
# どのようなプロジェクト環境でも単体で動かせるよう、標準ライブラリのみで構築されています。
# ==============================================================================

# Windows環境での絵文字文字化け即死防止
if sys.platform == "win32":
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8')

# ANSIエスケープシーケンスによるカラー装飾
CLR_RESET = "\033[0m"
CLR_BOLD = "\033[1m"
CLR_RED = "\033[91m"
CLR_GREEN = "\033[92m"
CLR_YELLOW = "\033[93m"
CLR_BLUE = "\033[94m"
CLR_CYAN = "\033[96m"
CLR_GRAY = "\033[90m"

# チェックボックス判定用の正規表現
# グループ1: 状態 ([ ], [/], [x], [X] 等)
# グループ2: タスク名
TODO_PATTERN = re.compile(r"^(\s*-\s*\[)([\s/xX])(\]\s+)(.*)$")
HEADER_PATTERN = re.compile(r"^(#+)\s+(.*)$")

def clear_screen():
    os.system("cls" if sys.platform == "win32" else "clear")

def find_target_file():
    """引数または自動探索でターゲットとするMarkdownファイルを決定します。"""
    # 1. コマンドライン引数があれば最優先
    if len(sys.argv) > 1:
        path = sys.argv[1]
        if os.path.exists(path):
            return path
        else:
            print(f"{CLR_RED}❌ 指定されたファイルが見つかりませんわ: {path}{CLR_RESET}")
            sys.exit(1)

    # 2. 自動探索優先順位
    search_patterns = [
        r".*checklist\.md$",
        r".*CHECKLIST\.md$",
        r"^todo\.md$",
        r"^TODO\.md$",
        r"^task\.md$",
        r"^TASK\.md$"
    ]
    
    current_files = os.listdir(".")
    for pattern in search_patterns:
        regex = re.compile(pattern, re.IGNORECASE)
        for f in current_files:
            if regex.match(f) and os.path.isfile(f):
                return f
                
    # 3. 見つからない場合はテンプレート作成の打診
    print(f"{CLR_YELLOW}🐾 カレントディレクトリにチェックリストファイルが見つかりませんわ。{CLR_RESET}")
    yn = input("👉 新規で 'TODO.md' を作成しますか？ (y/N): ").strip().lower()
    if yn.startswith('y'):
        default_content = (
            "# 🐾 Project Todo List\n\n"
            "## 🛠️ Main Tasks\n"
            "- [ ] はじめてのタスク (番号を入力して状態を切り替えます)\n"
            "- [ ] 二番目のタスク\n"
            "\n"
            "## 📝 Documents\n"
            "- [ ] ドキュメントの整備\n"
        )
        with open("TODO.md", "w", encoding="utf-8") as f:
            f.write(default_content)
        print(f"{CLR_GREEN}✓ 'TODO.md' を作成いたしました！💮{CLR_RESET}")
        return "TODO.md"
    else:
        print("処理を中断いたしました。")
        sys.exit(0)

def parse_markdown(filepath):
    """Markdownファイルを読み込み、非破壊的に保持しつつTODO項目を抽出します。"""
    with open(filepath, "r", encoding="utf-8") as f:
        lines = f.readlines()

    todo_items = []
    current_header = "General"
    
    for idx, line in enumerate(lines):
        # 見出しの追跡
        header_match = HEADER_PATTERN.match(line)
        if header_match:
            current_header = header_match.group(2)
            continue
            
        # TODO項目の判定
        todo_match = TODO_PATTERN.match(line)
        if todo_match:
            prefix = todo_match.group(1)
            status = todo_match.group(2)
            suffix = todo_match.group(3)
            text = todo_match.group(4)
            
            todo_items.append({
                "line_no": idx,
                "prefix": prefix,
                "status": status,
                "suffix": suffix,
                "text": text,
                "header": current_header
            })
            
    return lines, todo_items

def save_markdown(filepath, lines, todo_items):
    """変更されたTODO状態を元の行リストに反映して、非破壊的に上書き保存します。"""
    for item in todo_items:
        line_no = item["line_no"]
        prefix = item["prefix"]
        status = item["status"]
        suffix = item["suffix"]
        text = item["text"]
        # 行を再構成
        lines[line_no] = f"{prefix}{status}{suffix}{text}\n"
        
    with open(filepath, "w", encoding="utf-8") as f:
        f.writelines(lines)

def run_post_hook():
    """保存完了後に実行可能な後処理フックがあれば自動実行します。"""
    # 1. 環境変数による指定があれば優先実行
    post_cmd = os.environ.get("TANUKI_TODO_POST_CMD")
    if post_cmd:
        print(f"\n{CLR_CYAN}🚀 環境変数から指定された後処理を実行します: {post_cmd}{CLR_RESET}")
        subprocess.run(post_cmd, shell=True)
        return

    # 2. カレントディレクトリに rebuild_tanuki.py があれば自動実行
    if os.path.exists("rebuild_tanuki.py"):
        print(f"\n{CLR_CYAN}🚀 知識ベース再構築スクリプト (rebuild_tanuki.py) を自動実行しますわ🐾{CLR_RESET}")
        try:
            subprocess.run([sys.executable, "rebuild_tanuki.py"])
            print(f"{CLR_GREEN}✓ 知識ベースの再構築が完了いたしました！💮{CLR_RESET}")
        except Exception as e:
            print(f"{CLR_RED}❌ 再構築スクリプトの実行中にエラーが発生しました: {e}{CLR_RESET}")

def main():
    filepath = find_target_file()
    lines, todo_items = parse_markdown(filepath)
    
    if not todo_items:
        print(f"{CLR_RED}❌ ファイル内にチェックボックス項目 (- [ ], - [/], - [x]) が見つかりませんでしたわ🐾{CLR_RESET}")
        sys.exit(1)
        
    dirty = False
    
    while True:
        clear_screen()
        print(f"{CLR_BOLD}{CLR_CYAN}🐾 T.A.N.U.K.I. Interactive Todo Manager{CLR_RESET}")
        print(f"{CLR_GRAY}Target File: {os.path.abspath(filepath)}{CLR_RESET}")
        print("=" * 60)
        
        last_header = None
        for i, item in enumerate(todo_items):
            # 見出しが変わったらヘッダーを出力
            if item["header"] != last_header:
                last_header = item["header"]
                print(f"\n{CLR_BOLD}{CLR_BLUE}📋 {last_header}{CLR_RESET}")
                
            # ステータスの装飾
            status_char = item["status"]
            if status_char == " ":
                status_icon = f"[{CLR_RED}✗{CLR_RESET}]"
                task_style = ""
            elif status_char == "/":
                status_icon = f"[{CLR_YELLOW}⚡{CLR_RESET}]"
                task_style = CLR_YELLOW
            else: # x, X など
                status_icon = f"[{CLR_GREEN}✓{CLR_RESET}]"
                task_style = CLR_GRAY
                
            print(f"  {CLR_BOLD}{i+1:2d}){CLR_RESET} {status_icon} {task_style}{item['text']}{CLR_RESET}")
            
        print("\n" + "=" * 60)
        if dirty:
            print(f"{CLR_YELLOW}⚠️ 変更が未保存ですわ！[S]キーで保存してくださいね。{CLR_RESET}")
        print(f"👉 {CLR_BOLD}[番号]{CLR_RESET}: 状態をトグル | {CLR_BOLD}[S]{CLR_RESET}: 保存して終了 | {CLR_BOLD}[Q]{CLR_RESET}: 保存せずに終了")
        
        try:
            cmd = input("選択してください: ").strip().lower()
        except KeyboardInterrupt:
            print("\n終了しますわ🐾")
            sys.exit(0)
            
        if cmd == 's':
            save_markdown(filepath, lines, todo_items)
            print(f"\n{CLR_GREEN}✓ ファイルに保存いたしました！💮{CLR_RESET}")
            run_post_hook()
            break
        elif cmd == 'q':
            if dirty:
                confirm = input("⚠️ 変更が保存されていませんが、本当に終了しますか？ (y/N): ").strip().lower()
                if not confirm.startswith('y'):
                    continue
            print("保存せずに終了いたしました。")
            break
            
        # 番号が指定された場合
        if cmd.isdigit():
            idx = int(cmd) - 1
            if 0 <= idx < len(todo_items):
                item = todo_items[idx]
                current_status = item["status"]
                
                # トグル状態の遷移: 空白 (未完了) -> / (進行中) -> x (完了) -> 空白
                if current_status == " ":
                    next_status = "/"
                elif current_status == "/":
                    next_status = "x"
                else:
                    next_status = " "
                    
                item["status"] = next_status
                dirty = True
            else:
                print(f"{CLR_RED}範囲外の番号ですわ🐾{CLR_RESET}")
                input("Enterキーで続行...")
        elif cmd != '':
            print(f"{CLR_RED}無効な入力ですわ🐾{CLR_RESET}")
            input("Enterキーで続行...")

if __name__ == "__main__":
    main()
