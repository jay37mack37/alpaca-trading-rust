use criterion::{criterion_group, criterion_main, Criterion};
use rusqlite::{params, Connection};

fn benchmark_inserts(c: &mut Criterion) {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE broker_positions (
            id INTEGER PRIMARY KEY,
            credential_id TEXT,
            symbol TEXT,
            asset_class TEXT,
            side TEXT,
            quantity REAL,
            avg_entry_price REAL,
            market_value REAL,
            current_price REAL,
            unrealized_pl REAL,
            unrealized_plpc REAL,
            synced_at TEXT,
            raw_json TEXT
        )",
    ).unwrap();

    let num_positions = 100;

    let mut group = c.benchmark_group("inserts");

    group.bench_function("n_plus_1", |b| b.iter(|| {
        let tx = conn.transaction().unwrap();
        tx.execute("DELETE FROM broker_positions", []).unwrap();
        for i in 0..num_positions {
            tx.execute(
                "INSERT INTO broker_positions (
                    credential_id, symbol, asset_class, side, quantity, avg_entry_price, market_value,
                    current_price, unrealized_pl, unrealized_plpc, synced_at, raw_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    "cred_1",
                    format!("SYM{}", i),
                    "equity",
                    "long",
                    10.0,
                    100.0,
                    1000.0,
                    105.0,
                    50.0,
                    0.05,
                    "2023-01-01T00:00:00Z",
                    "{}",
                ],
            ).unwrap();
        }
        tx.commit().unwrap();
    }));

    group.bench_function("prepared_stmt", |b| b.iter(|| {
        let tx = conn.transaction().unwrap();
        tx.execute("DELETE FROM broker_positions", []).unwrap();
        let mut stmt = tx.prepare(
            "INSERT INTO broker_positions (
                credential_id, symbol, asset_class, side, quantity, avg_entry_price, market_value,
                current_price, unrealized_pl, unrealized_plpc, synced_at, raw_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"
        ).unwrap();
        for i in 0..num_positions {
            stmt.execute(
                params![
                    "cred_1",
                    format!("SYM{}", i),
                    "equity",
                    "long",
                    10.0,
                    100.0,
                    1000.0,
                    105.0,
                    50.0,
                    0.05,
                    "2023-01-01T00:00:00Z",
                    "{}",
                ],
            ).unwrap();
        }
        drop(stmt);
        tx.commit().unwrap();
    }));

    group.finish();
}

criterion_group!(benches, benchmark_inserts);
criterion_main!(benches);
