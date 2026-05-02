use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rusqlite::{params, Connection};

fn bench_db_inserts(c: &mut Criterion) {
    let mut group = c.benchmark_group("db_inserts");

    group.bench_function("unprepared_inserts", |b| {
        b.iter(|| {
            let mut conn = Connection::open_in_memory().unwrap();
            conn.execute(
                "CREATE TABLE broker_orders (
                    credential_id TEXT, order_id TEXT, client_order_id TEXT, symbol TEXT, side TEXT, order_type TEXT, order_class TEXT,
                    status TEXT, quantity REAL, filled_qty REAL, filled_avg_price REAL, time_in_force TEXT, submitted_at INTEGER,
                    updated_at INTEGER, synced_at INTEGER, raw_json TEXT
                )",
                [],
            ).unwrap();

            let tx = conn.transaction().unwrap();
            for i in 0..1000 {
                tx.execute(
                    "INSERT INTO broker_orders (
                        credential_id, order_id, client_order_id, symbol, side, order_type, order_class,
                        status, quantity, filled_qty, filled_avg_price, time_in_force, submitted_at,
                        updated_at, synced_at, raw_json
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                    params![
                        "cred1", format!("order{}", i), "client1", "AAPL", "buy", "market", "simple",
                        "filled", 10.0, 10.0, 150.0, "day", 1600000000, 1600000000, 1600000000, "{}"
                    ],
                ).unwrap();
            }
            tx.commit().unwrap();
        });
    });

    group.bench_function("prepared_inserts", |b| {
        b.iter(|| {
            let mut conn = Connection::open_in_memory().unwrap();
            conn.execute(
                "CREATE TABLE broker_orders (
                    credential_id TEXT, order_id TEXT, client_order_id TEXT, symbol TEXT, side TEXT, order_type TEXT, order_class TEXT,
                    status TEXT, quantity REAL, filled_qty REAL, filled_avg_price REAL, time_in_force TEXT, submitted_at INTEGER,
                    updated_at INTEGER, synced_at INTEGER, raw_json TEXT
                )",
                [],
            ).unwrap();

            let tx = conn.transaction().unwrap();
            {
                let mut stmt = tx.prepare(
                    "INSERT INTO broker_orders (
                        credential_id, order_id, client_order_id, symbol, side, order_type, order_class,
                        status, quantity, filled_qty, filled_avg_price, time_in_force, submitted_at,
                        updated_at, synced_at, raw_json
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"
                ).unwrap();
                for i in 0..1000 {
                    stmt.execute(
                        params![
                            "cred1", format!("order{}", i), "client1", "AAPL", "buy", "market", "simple",
                            "filled", 10.0, 10.0, 150.0, "day", 1600000000, 1600000000, 1600000000, "{}"
                        ],
                    ).unwrap();
                }
            }
            tx.commit().unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_db_inserts);
criterion_main!(benches);
