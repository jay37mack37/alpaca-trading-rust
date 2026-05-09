use rusqlite::{Connection, params};

fn main() -> anyhow::Result<()> {
    let conn = Connection::open("data/autostonks.db")?;
    let mut stmt = conn.prepare("SELECT id, name, starting_cash, cash_balance, use_shared_cash FROM strategies")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, i64>(4)? != 0,
        ))
    })?;

    println!("Strategies in DB:");
    for row in rows {
        let (id, name, starting_cash, cash_balance, use_shared_cash) = row?;
        println!("ID: {}, Name: {}, Starting Cash: {}, Balance: {}, Shared: {}", id, name, starting_cash, cash_balance, use_shared_cash);
    }

    let mut stmt = conn.prepare("SELECT strategy_id, symbol, decision, narrative FROM intelligence_logs ORDER BY timestamp DESC LIMIT 10")?;
    let logs = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    println!("\nRecent Intelligence Logs:");
    for log in logs {
        let (sid, sym, dec, nar) = log?;
        println!("Strategy: {}, Symbol: {}, Decision: {}, Narrative: {}", sid, sym, dec, nar);
    }

    Ok(())
}
