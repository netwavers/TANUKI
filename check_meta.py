import sqlite3
conn = sqlite3.connect('knowledge.db')
cursor = conn.cursor()
cursor.execute("SELECT path FROM file_meta WHERE path LIKE '%20260414_Lyric_Pipeline_Refinement%'")
row = cursor.fetchone()
if row:
    print(f"Meta exists for: {row[0]}")
else:
    print("Meta NOT FOUND.")
