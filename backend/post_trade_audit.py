import sqlite3
import os
import json
from datetime import datetime
from collections import defaultdict

DB_PATH = 'c:/Users/javel/OneDrive/Documents/GitHub/alpaca-trading-rust/backend/data/autostonks.db'

def parse_iso_datetime(dt_str):
    if not dt_str: return None
    try:
        # 2026-04-24T13:58:03.415961+00:00
        clean_str = dt_str.replace('T', ' ').split('+')[0].split('.')[0]
        return datetime.strptime(clean_str, '%Y-%m-%d %H:%M:%S')
    except Exception as e:
        return None

def run_audit():
    if not os.path.exists(DB_PATH):
        print(f"Error: Database not found at {DB_PATH}")
        return

    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    
    # 1. Load Trade Logs
    trades = conn.execute("SELECT * FROM trade_log ORDER BY strategy_id, symbol, executed_at").fetchall()
    if not trades:
        print("No trades found in trade_log.")
        return

    # 2. Pair Entry and Exit Trades
    completed_trades = []
    active_entries = {}

    for row in trades:
        key = (row['strategy_id'], row['symbol'])
        if row['side'] == 'buy':
            active_entries[key] = row
        elif row['side'] == 'sell' and key in active_entries:
            entry = active_entries.pop(key)
            pnl = row['realized_pnl']
            if pnl is None:
                pnl = (row['price'] - entry['price']) * row['quantity']
                
            completed_trades.append({
                'strategy_id': row['strategy_id'],
                'symbol': row['symbol'],
                'asset_type': row['asset_type'],
                'entry_time': entry['executed_at'],
                'exit_time': row['executed_at'],
                'entry_price': entry['price'],
                'exit_price': row['price'],
                'pnl': pnl,
                'quantity': row['quantity']
            })

    if not completed_trades:
        print("No completed (Buy -> Sell) trades found.")
    else:
        print("\n" + "="*70)
        print("POST-TRADE PERFORMANCE AUDIT (LEARNING MODE)")
        print("="*70)
        
        strat_performance = defaultdict(lambda: {
            'efficiencies': [],
            'hourly_pnl': defaultdict(list),
            'total_trades': 0
        })

        for idx, trade in enumerate(completed_trades):
            strat_id = trade['strategy_id']
            strat_performance[strat_id]['total_trades'] += 1
            
            # Extract hour for timing analysis
            dt = parse_iso_datetime(trade['entry_time'])
            if dt:
                strat_performance[strat_id]['hourly_pnl'][dt.hour].append(trade['pnl'])

            # Peak Analysis
            a_type = trade['asset_type'].lower()
            is_option = 'option' in a_type
            table = "option_snapshots" if is_option else "market_snapshots"
            symbol_col = "contract_symbol" if is_option else "symbol"
            price_col = "bid" if is_option else "price"
            
            cursor = conn.execute(
                f"SELECT {price_col} FROM {table} WHERE {symbol_col} = ? AND captured_at BETWEEN ? AND ? ORDER BY {price_col} DESC LIMIT 1",
                (trade['symbol'], trade['entry_time'], trade['exit_time'])
            )
            peak = cursor.fetchone()
            
            if peak and peak[0] is not None:
                max_price = peak[0]
                theoretical_max_pnl = (max_price - trade['entry_price']) * trade['quantity']
                if theoretical_max_pnl > 0:
                    efficiency = (trade['pnl'] / theoretical_max_pnl)
                    strat_performance[strat_id]['efficiencies'].append(efficiency)

        # 3. Save Learning Results to Database
        for strat_id, stats in strat_performance.items():
            # Calculate Best Hours (hours with positive avg PnL)
            hour_avgs = {h: sum(pnls)/len(pnls) for h, pnls in stats['hourly_pnl'].items()}
            best_hours = [h for h, avg in hour_avgs.items() if avg > 0]
            
            avg_efficiency = sum(stats['efficiencies']) / len(stats['efficiencies']) if stats['efficiencies'] else 0.0
            
            learning_data = {
                'last_audit': datetime.now().isoformat(),
                'best_hours': best_hours,
                'avg_efficiency': avg_efficiency,
                'total_audited_trades': stats['total_trades']
            }
            
            print(f"Strategy {strat_id} learned:")
            print(f"  Best Trading Hours: {best_hours}")
            print(f"  Avg Trade Efficiency: {avg_efficiency:.2%}")
            
            conn.execute(
                "UPDATE strategies SET performance_stats_json = ? WHERE id = ?",
                (json.dumps(learning_data), strat_id)
            )
            print(f"  Feedback loop updated for {strat_id}.")

    conn.commit()
    conn.close()

if __name__ == "__main__":
    run_audit()
