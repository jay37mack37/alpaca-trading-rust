use rusqlite::Connection;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = "data/autostonks.db";
    if !Path::new(db_path).exists() {
        println!("Database not found at {}", db_path);
        return Ok(());
    }

    let conn = Connection::open(db_path)?;
    
    let targets = vec![
        "asddasdasdasdasd",
        "$asddasdasdasdasd",
        "So like what do u want",
        "asd",
        "aw"
    ];

    for target in targets {
        let deleted = conn.execute(
            "DELETE FROM strategies WHERE name = ?1",
            [target],
        )?;
        println!("Deleted {} strategy records matching name: '{}'", deleted, target);
        
        // Also delete from trade_log if any
        let trade_deleted = conn.execute(
            "DELETE FROM trade_log WHERE strategy_id IN (SELECT id FROM strategies WHERE name = ?1)",
            [target],
        )?;
        if trade_deleted > 0 {
            println!("Deleted {} trade log records for strategy: '{}'", trade_deleted, target);
        }
    }

    println!("Cleanup complete!");
    Ok(())
}
