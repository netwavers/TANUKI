import sqlite3
import os

def populate_clusters():
    db_path = "knowledge.db"
    if not os.path.exists(db_path):
        print(f"Error: {db_path} does not exist.")
        return

    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    # 1. すでに登録されているクラスタのIDを取得
    cursor.execute("SELECT id FROM clusters")
    existing_clusters = {row[0] for row in cursor.fetchall()}

    # 2. ノードからユニークな context_path を取得
    cursor.execute("SELECT DISTINCT context_path FROM nodes")
    context_paths = {row[0] for row in cursor.fetchall() if row[0]}

    # 3. 未登録のクラスタを登録
    inserted_count = 0
    for path in context_paths:
        if path not in existing_clusters:
            # タイトルはフォルダ名の末尾
            title = path.split('/')[-1] if '/' in path else path
            title = title.split(' > ')[-1] if ' > ' in title else title
            if not title:
                title = "Root Cluster"
            
            # clusters テーブルの構造:
            # id TEXT PRIMARY KEY, parent_id TEXT, title TEXT, summary TEXT, navigation_criteria TEXT
            cursor.execute(
                "INSERT INTO clusters (id, parent_id, title, summary, navigation_criteria) VALUES (?, ?, ?, ?, ?)",
                (path, "root", title, f"Knowledge cluster for {path}", "Standard context navigation")
            )
            inserted_count += 1

    conn.commit()
    print(f"Successfully inserted {inserted_count} new clusters into database.")
    
    # 登録後の合計数を確認
    cursor.execute("SELECT count(*) FROM clusters")
    total_clusters = cursor.execute("SELECT count(*) FROM clusters").fetchone()[0]
    print(f"Total clusters in database: {total_clusters}")
    
    conn.close()

if __name__ == "__main__":
    populate_clusters()
