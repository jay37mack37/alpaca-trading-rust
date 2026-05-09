use rusqlite::Connection;
use std::path::Path;

fn main() {
    let db_path = "autostonks.db";
    if !Path::new(db_path).exists() {
        println!("Database not found at {}", db_path);
        return;
    }

    let conn = Connection::open(db_path).unwrap();
    let mut stmt = conn.prepare("SELECT timestamp, source, symbol, event_type, narrative FROM strategy_logs ORDER BY timestamp DESC LIMIT 10").unwrap();
    
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    }).unwrap();

    println!("--- RECENT STRATEGY LOGS ---");
    for row in rows {
        let (ts, src, sym, evt, msg) = row.unwrap();
        println!("[{}] {} | {} -> {}: {}", ts, src, sym, evt, msg);
    }
}
