use rusqlite::{params, Connection};
use std::time::Instant;

fn main() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE option_snapshots (
            id TEXT PRIMARY KEY,
            underlying_symbol TEXT,
            provider TEXT,
            contract_symbol TEXT,
            option_type TEXT,
            expiration TEXT,
            strike REAL,
            bid REAL,
            ask REAL,
            last REAL,
            implied_volatility REAL,
            open_interest INTEGER,
            volume INTEGER,
            in_the_money INTEGER,
            delta REAL,
            gamma REAL,
            theta REAL,
            vega REAL,
            moneyness REAL,
            captured_at INTEGER,
            raw_json TEXT
        )",
        [],
    )
    .unwrap();

    let num_records = 10_000;

    // Simulate current behavior
    let start_unoptimized = Instant::now();
    let tx = conn.transaction().unwrap();
    for i in 0..num_records {
        tx.execute(
            "INSERT INTO option_snapshots (
                id, underlying_symbol, provider, contract_symbol, option_type, expiration, strike,
                bid, ask, last, implied_volatility, open_interest, volume, in_the_money,
                delta, gamma, theta, vega, moneyness, captured_at, raw_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                format!("unopt_{}", i), "AAPL", "alpaca", "AAPL210115C00100000", "call", "2021-01-15", 100.0,
                1.5, 1.6, 1.55, 0.2, 1000, 500, 1,
                0.5, 0.1, -0.05, 0.2, 1.05, 1610000000, "{}"
            ],
        ).unwrap();
    }
    tx.commit().unwrap();
    let duration_unoptimized = start_unoptimized.elapsed();

    // Simulate optimized behavior
    let start_optimized = Instant::now();
    let tx = conn.transaction().unwrap();
    {
        let mut stmt = tx.prepare(
            "INSERT INTO option_snapshots (
                id, underlying_symbol, provider, contract_symbol, option_type, expiration, strike,
                bid, ask, last, implied_volatility, open_interest, volume, in_the_money,
                delta, gamma, theta, vega, moneyness, captured_at, raw_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)"
        ).unwrap();
        for i in 0..num_records {
            stmt.execute(params![
                format!("opt_{}", i),
                "AAPL",
                "alpaca",
                "AAPL210115C00100000",
                "call",
                "2021-01-15",
                100.0,
                1.5,
                1.6,
                1.55,
                0.2,
                1000,
                500,
                1,
                0.5,
                0.1,
                -0.05,
                0.2,
                1.05,
                1610000000,
                "{}"
            ])
            .unwrap();
        }
    }
    tx.commit().unwrap();
    let duration_optimized = start_optimized.elapsed();

    println!("Unoptimized: {:?}", duration_unoptimized);
    println!("Optimized:   {:?}", duration_optimized);
    let speedup = duration_unoptimized.as_secs_f64() / duration_optimized.as_secs_f64();
    println!("Speedup: {:.2}x", speedup);
}
