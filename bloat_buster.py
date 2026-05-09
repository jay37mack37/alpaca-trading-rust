import sqlite3
import os
from datetime import datetime, timedelta

def prune_db(db_path, days=7):
    if not os.path.exists(db_path):
        print(f"Database not found at {db_path}")
        return

    print(f"Pruning {db_path} (keeping last {days} days)...")
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    cutoff = (datetime.now() - timedelta(days=days)).isoformat()
    
    tables_to_prune = [
        "market_snapshots",
        "option_snapshots",
        "intelligence_logs",
        "account_balance_snapshots"
    ]

    for table in tables_to_prune:
        try:
            cursor.execute(f"DELETE FROM {table} WHERE captured_at < ?", (cutoff,))
            print(f"Pruned {cursor.rowcount} rows from {table}")
        except Exception as e:
            # intelligence_logs uses 'timestamp' instead of 'captured_at'
            if table == "intelligence_logs":
                try:
                    cursor.execute(f"DELETE FROM intelligence_logs WHERE timestamp < ?", (cutoff,))
                    print(f"Pruned {cursor.rowcount} rows from {table}")
                    continue
                except:
                    pass
            print(f"Could not prune {table}: {e}")

    conn.commit()
    print("Pruning complete. Checking if vacuum is feasible...")
    # Skipping heavy vacuum if space is tight, but we'll try it since we have 88GB free
    try:
        print("Vacuuming database to reclaim disk space...")
        conn.execute("VACUUM")
    except Exception as e:
        print(f"Vacuum failed (likely disk space): {e}")
    conn.close()
    print(f"Finished pruning {db_path}")

if __name__ == "__main__":
    # Prune the main backend database
    prune_db("backend/data/autostonks.db")
    
    # Check if root data/autostonks.db exists and delete it if it's redundant
    # (The backend uses backend/data/autostonks.db)
    root_db = "data/autostonks.db"
    if os.path.exists(root_db):
        print(f"Removing redundant root database: {root_db}")
        os.remove(root_db)

    print("Cleanup complete!")
