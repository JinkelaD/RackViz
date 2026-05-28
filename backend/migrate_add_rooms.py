import sqlite3
import os

DB = os.path.join(os.path.dirname(__file__), "rackviz.db")
conn = sqlite3.connect(DB)
cur = conn.cursor()

cur.executescript("""
CREATE TABLE IF NOT EXISTS rooms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(100) NOT NULL,
    location VARCHAR(200) DEFAULT '',
    sort_order INTEGER DEFAULT 0
);

ALTER TABLE racks ADD COLUMN room_id INTEGER REFERENCES rooms(id);
""")

conn.commit()
conn.close()
print("Migration done: rooms table created, room_id column added to racks.")
