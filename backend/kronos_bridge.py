import sys
import os
import torch
import pandas as pd
import yfinance as yf
from fastapi import FastAPI, HTTPException
from datetime import datetime, timedelta

# Add kronos_engine (in root) to path
current_dir = os.path.dirname(os.path.abspath(__file__))
root_dir = os.path.dirname(current_dir)
sys.path.append(os.path.join(root_dir, "kronos_engine"))

try:
    from model import Kronos, KronosTokenizer, KronosPredictor
except ImportError:
    # Fallback
    sys.path.append(root_dir)
    from kronos_engine.model import Kronos, KronosTokenizer, KronosPredictor

app = FastAPI()

# Global model state
tokenizer = None
model = None
predictor = None

def init_model():
    global tokenizer, model, predictor
    print("Loading Kronos Foundation Model (NeoQuasar/Kronos-small)...")
    tokenizer = KronosTokenizer.from_pretrained("NeoQuasar/Kronos-Tokenizer-base")
    model = Kronos.from_pretrained("NeoQuasar/Kronos-small")
    predictor = KronosPredictor(model, tokenizer, max_context=512)
    print("Kronos Model Loaded Successfully!")

@app.on_event("startup")
async def startup_event():
    init_model()

@app.get("/score/{symbol}")
async def get_score(symbol: str):
    try:
        print(f"Calculating AI Score for {symbol}...")
        ticker = yf.Ticker(symbol)
        df = ticker.history(period="5d", interval="5m")
        
        if df.empty:
            raise HTTPException(status_code=404, detail="No data found for symbol")

        df = df.rename(columns={
            'Open': 'open', 'High': 'high', 'Low': 'low', 'Close': 'close', 'Volume': 'volume'
        })
        df['amount'] = df['close'] * df['volume']
        
        x_timestamp = df.index.to_series()
        pred_len = 12
        last_time = x_timestamp.iloc[-1]
        y_timestamp = pd.Series([last_time + timedelta(minutes=5 * i) for i in range(1, pred_len + 1)])

        input_df = df[['open', 'high', 'low', 'close', 'volume', 'amount']].copy()
        
        with torch.no_grad():
            pred_df = predictor.predict(
                df=input_df,
                x_timestamp=x_timestamp,
                y_timestamp=y_timestamp,
                pred_len=pred_len,
                T=1.0,
                top_p=0.9,
                sample_count=1
            )

        current_price = df['close'].iloc[-1]
        predicted_price = pred_df['close'].iloc[-1]
        change = (predicted_price - current_price) / current_price
        
        score = 0.5 + (change * 20.0)
        score = max(0.1, min(0.9, score))
        trend = "BULLISH" if change > 0 else "BEARISH"
        
        print(f"Score for {symbol}: {score:.4f} ({trend})")
        return {
            "symbol": symbol,
            "trend": trend,
            "confidence": round(float(score), 4),
            "predicted_change_pct": round(float(change * 100), 2)
        }

    except Exception as e:
        print(f"Error scoring {symbol}: {e}")
        return {
            "symbol": symbol,
            "trend": "NEUTRAL",
            "confidence": 0.5,
            "error": str(e)
        }

@app.get("/health")
async def health():
    return {"status": "ready" if model is not None else "loading"}

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)
