import sqlite3
conn = sqlite3.connect('knowledge.db')
cursor = conn.cursor()
cursor.execute("SELECT title, content FROM nodes WHERE source_path LIKE '%20260414_Lyric_Pipeline_Refinement%'")
rows = cursor.fetchall()
for title, content in rows:
    print(f"Title length: {len(title)}")
    print(f"Content length: {len(content)}")
    print(f"First 50 chars of content: {repr(content[:50])}")
