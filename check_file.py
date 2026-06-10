import sqlite3
conn = sqlite3.connect('knowledge.db')
cursor = conn.cursor()
cursor.execute("SELECT title FROM nodes WHERE source_path LIKE '%20260414_Lyric_Pipeline_Refinement%'")
rows = cursor.fetchall()
print(f"Nodes found for this file: {len(rows)}")
for r in rows:
    print(f"- {r[0]}")
