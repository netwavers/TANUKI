import sqlite3
conn = sqlite3.connect('knowledge.db')
cursor = conn.cursor()
cursor.execute("SELECT title, content FROM nodes WHERE source_path LIKE '%20260414_Lyric_Pipeline_Refinement%'")
rows = cursor.fetchall()
print(f"Nodes found: {len(rows)}")
for title, content in rows:
    print(f"- Title: {repr(title)}")
