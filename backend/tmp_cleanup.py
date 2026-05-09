import sqlite3
import os

db_path = "backend/data/autostonks.db"
if not os.path.exists(db_path):
    print(f"Database not found at {db_path}")
    exit(1)

conn = sqlite3.connect(db_path)
cursor = conn.cursor()

targets = [
    "asddasdasdasdasd",
    "$asddasdasdasdasd",
    "So like what do u want",
    "asd",
    "aw"
]

for target in targets:
    cursor.execute("DELETE FROM strategies WHERE name = ?", (target,))
    print(f"Deleted {cursor.rowcount} strategy records matching name: '{target}'")
    
    # Delete from strategy_positions
    cursor.execute("DELETE FROM strategy_positions WHERE strategy_id NOT IN (SELECT id FROM strategies)")
    if cursor.rowcount > 0:
        print(f"Cleaned up {cursor.rowcount} orphaned positions.")

    # Delete from trade_log
    cursor.execute("DELETE FROM trade_log WHERE strategy_id NOT IN (SELECT id FROM strategies)")
    if cursor.rowcount > 0:
        print(f"Cleaned up {cursor.rowcount} orphaned trade logs.")

conn.commit()
conn.close()
print("Cleanup complete!")
