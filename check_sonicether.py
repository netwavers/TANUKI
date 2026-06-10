import sqlite3
import os

db_path = r'd:\Projects\PyProjects\SonicEther\workspace\sonicether.db'
if not os.path.exists(db_path):
    print(f"DB not found at {db_path}")
    exit(1)

conn = sqlite3.connect(db_path)
tables = [row[0] for row in conn.execute("SELECT name FROM sqlite_master WHERE type='table'")]
print(f"Tables: {tables}")

for table in tables:
    print(f"\n--- {table} ---")
    try:
        rows = conn.execute(f"SELECT * FROM {table} LIMIT 10").fetchall()
        cols = [description[0] for description in conn.execute(f"SELECT * FROM {table} LIMIT 1").description]
        print(cols)
        for row in rows:
            print(row)
    except Exception as e:
        print(f"Error reading {table}: {e}")
