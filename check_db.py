import sqlite3
import sys

conn = sqlite3.connect('knowledge.db')
cursor = conn.cursor()
cursor.execute("SELECT title, content FROM nodes")
rows = cursor.fetchall()

print(f"Total nodes: {len(rows)}")
found = False
for title, content in rows:
    if "VRAMの亡霊" in content:
        print(f"FOUND in node: {title}")
        found = True

if not found:
    print("NOT FOUND in any node.")
