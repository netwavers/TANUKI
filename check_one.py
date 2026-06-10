import sqlite3
conn = sqlite3.connect('knowledge.db')
count = conn.execute("SELECT COUNT(*) FROM nodes WHERE source_path LIKE '%20260414_Lyric_Pipeline_Refinement%'").fetchone()[0]
print(f"Nodes for 20260414: {count}")
meta = conn.execute("SELECT COUNT(*) FROM file_meta WHERE path LIKE '%20260414_Lyric_Pipeline_Refinement%'").fetchone()[0]
print(f"Meta for 20260414: {meta}")
