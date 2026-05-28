import sqlite3, os

DB = os.path.join(os.path.dirname(__file__), "rackviz.db")
conn = sqlite3.connect(DB)
conn.execute("ALTER TABLE racks ADD COLUMN location VARCHAR(200) DEFAULT ''")
conn.commit()
conn.close()
print("Migration done: location column added to racks.")
