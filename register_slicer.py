import sqlite3
import os

def register_node(title, file_path):
    if not os.path.exists(file_path):
        print(f"File not found: {file_path}")
        return
        
    with open(file_path, "r", encoding="utf-8") as f:
        content = f.read()
        
    conn = sqlite3.connect('knowledge.db')
    cursor = conn.cursor()
    
    # 重複チェック
    cursor.execute("SELECT count(*) FROM nodes WHERE title = ?", (title,))
    if cursor.fetchone()[0] > 0:
        cursor.execute("UPDATE nodes SET content = ? WHERE title = ?", (content, title))
        print(f"Updated node: {title}")
    else:
        cursor.execute("INSERT INTO nodes (title, content) VALUES (?, ?)", (title, content))
        print(f"Registered new node: {title}")
        
    conn.commit()
    conn.close()

if __name__ == "__main__":
    # 技術仕様書の登録
    register_node("Tanuki Slicer Technical Specification", "../TanukiParser/SLICER_TECHNICAL_SPEC.md")
    # Python Slicer EBNFの登録
    register_node("Python Slicer EBNF Definition", "../TanukiParser/python_slicer.ebnf")
    # Markdown Slicer EBNFの登録
    register_node("Markdown Slicer EBNF Definition", "../TanukiParser/markdown_slicer.ebnf")
