import sqlite3
conn = sqlite3.connect('knowledge.db')
conn.execute("DELETE FROM file_meta WHERE path LIKE '%20260414_Lyric_Pipeline_Refinement%'")
conn.execute("DELETE FROM nodes WHERE source_path LIKE '%20260414_Lyric_Pipeline_Refinement%'")
conn.commit()
print("Cleaned up meta and nodes for 20260414.")
