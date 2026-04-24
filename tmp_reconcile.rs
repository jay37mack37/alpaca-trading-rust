use rusqlite::{params, Connection};
use uuid::Uuid;
use chrono::Utc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = "backend/data/trading.db";
    let conn = Connection::open(db_path)?;

    println!("Reconciling all open positions to trade history...");

    let mut stmt = conn.prepare("SELECT strategy_id, symbol, underlying_symbol, instrument_symbol, asset_type, quantity, average_price, market_price, multiplier, option_structure_preset, option_type, expiration, strike, legs_json, hold_intent, exit_logic, planned_exit, buy_logic, entry_math, entry_ai FROM strategy_positions")?;
    
    let positions = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?, // strategy_id
            row.get::<_, String>(1)?, // symbol
            row.get::<_, String>(2)?, // underlying
            row.get::<_, String>(3)?, // instrument
            row.get::<_, String>(4)?, // asset_type
            row.get::<_, f64>(5)?,    // qty
            row.get::<_, f64>(6)?,    // avg_price
            row.get::<_, f64>(7)?,    // market_price
            row.get::<_, f64>(8)?,    // multiplier
            row.get::<_, Option<String>>(9)?, // preset
            row.get::<_, Option<String>>(10)?, // type
            row.get::<_, Option<String>>(11)?, // exp
            row.get::<_, Option<f64>>(12)?,   // strike
            row.get::<_, String>(13)?,        // legs
            row.get::<_, Option<String>>(14)?, // intent
            row.get::<_, Option<String>>(15)?, // exit_logic
            row.get::<_, Option<String>>(16)?, // planned_exit
            row.get::<_, Option<String>>(17)?, // buy_logic
            row.get::<_, Option<String>>(18)?, // math
            row.get::<_, Option<f64>>(19)?,    // ai
        ))
    })?;

    let now = Utc::now().to_rfc3339();

    for pos in positions {
        let (strategy_id, symbol, underlying, instrument, asset_type, qty, avg, mkt, multi, preset, opt_type, exp, strike, legs, intent, exit_logic, planned, buy_logic, math, ai) = pos?;
        
        let trade_id = Uuid::new_v4().to_string();
        let pnl = (mkt - avg) * qty * multi;

        println!("Moving {} ({}) to history. P/L: ${:.2}", instrument, strategy_id, pnl);

        conn.execute(
            "INSERT INTO trade_log (
                id, strategy_id, symbol, underlying_symbol, instrument_symbol, asset_type, side,
                quantity, price, multiplier, option_structure_preset, option_type, expiration, strike,
                legs_json, provider, execution_mode, reason, realized_pnl, executed_at,
                hold_intent, exit_logic, planned_exit, buy_logic, entry_math, entry_ai
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'sell', ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 'alpaca', 'manual_reconciliation', 'BULK RECONCILIATION MIGRATION', ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
            params![
                trade_id, strategy_id, symbol, underlying, instrument, asset_type,
                qty, mkt, multi, preset, opt_type, exp, strike, legs,
                pnl, now, intent, exit_logic, planned, buy_logic, math, ai
            ],
        )?;

        conn.execute("DELETE FROM strategy_positions WHERE strategy_id = ?1 AND instrument_symbol = ?2", params![strategy_id, instrument])?;
    }

    println!("Done.");
    Ok(())
}
