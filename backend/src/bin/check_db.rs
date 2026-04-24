use rusqlite::Connection;
use std::collections::BTreeSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = "data/autostonks.db";
    let conn = Connection::open(db_path)?;
    
    let mut stmt = conn.prepare("SELECT DISTINCT execution_mode FROM trade_log")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    
    let mut modes = BTreeSet::new();
    for mode in rows {
        modes.insert(mode?);
    }
    
    println!("Distinct execution modes in trade_log:");
    for mode in &modes {
        println!(" - {}", mode);
    }

    let mut stmt_pos = conn.prepare("SELECT DISTINCT execution_mode FROM strategy_positions")?;
    let rows_pos = stmt_pos.query_map([], |row| row.get::<_, String>(0))?;
    for mode in rows_pos {
        modes.insert(mode?);
    }
    
    println!("\nCombined distinct execution modes:");
    for mode in &modes {
        println!(" - {}", mode);
    }
    
    Ok(())
}
