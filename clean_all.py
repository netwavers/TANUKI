import sqlite3
conn = sqlite3.connect('knowledge.db')
conn.execute("DELETE FROM file_meta")
conn.execute("DELETE FROM nodes")
conn.commit()
print("CLEANED ALL.")
