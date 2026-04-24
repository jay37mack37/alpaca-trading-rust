use crate::models::{
    Candle, Quote, SignalAction, StrategyRecord, StrategySignal, TradeLeg, TradeSide,
};
use async_trait::async_trait;
use crate::strategies::{TradingStrategy, hold, buy};
use crate::AppState;
use crate::services::corporate_actions::CorporateActionsService;
use chrono::{Utc, Duration};

pub struct DistributionSniperStrategy;

#[async_trait]
impl TradingStrategy for DistributionSniperStrategy {
    async fn evaluate(
        &self,
        state: &AppState,
        _strategy: &StrategyRecord,
        _candles: &[Candle],
        quote: &Quote,
        options: &[crate::models::OptionContractSnapshot],
        position: Option<&crate::models::PositionRecord>,
        _kronos_score: Option<f64>,
    ) -> StrategySignal {
        // 1. If we have a position, we are waiting for the ex-date to pass
        if position.is_some() {
            return hold("Hedged position active. Waiting for dividend capture.");
        }

        // 2. Fetch dividend info
        crate::agents::broadcast_audit_log(
            state,
            crate::logger::SystemEvent::now(
                crate::logger::SystemSource::System,
                Some(_strategy.id.clone()),
                quote.symbol.clone(),
                crate::logger::SystemEventType::Scan,
                format!("Price: ${:.2}", quote.price),
                0.5,
                format!("DIVIDEND SCAN: Checking yield capture feasibility for {}.", quote.symbol),
            )
        );

        let dividend_info = match CorporateActionsService::fetch_dividend_info(&state.http, &quote.symbol).await {
            Ok(Some(info)) => info,
            _ => {
                crate::agents::broadcast_audit_log(
                    state,
                    crate::logger::SystemEvent::now(
                        crate::logger::SystemSource::System,
                        Some(_strategy.id.clone()),
                        quote.symbol.clone(),
                        crate::logger::SystemEventType::Scan,
                        "N/A".to_string(),
                        0.5,
                        format!("NO DIVIDEND: {} has no upcoming payouts in database.", quote.symbol),
                    )
                );
                return hold("No upcoming dividend found");
            }
        };

        let now = Utc::now();
        let time_to_ex = dividend_info.ex_dividend_date.signed_duration_since(now);

        // 3. Trigger: 24 hours prior to ex-date
        if time_to_ex > Duration::hours(0) && time_to_ex < Duration::hours(24) {
            // Find a Deep ITM Put (Delta > 0.85 approx, or just Strike > Price + 5%)
            let target_strike = quote.price * 1.10; // 10% ITM Put
            
            // Find the contract in snapshots
            let best_put = options.iter()
                .filter(|o| o.contract_symbol.contains('P') && o.strike >= target_strike)
                .min_by(|a, b| a.strike.partial_cmp(&b.strike).unwrap());

            if let Some(put) = best_put {
                let put_price = put.ask.unwrap_or(put.last.unwrap_or(0.0));
                let intrinsic_value = (put.strike - quote.price).max(0.0);
                let time_value = put_price - intrinsic_value;

                // The math: Cost of the Put (time value) < Dividend Amount
                // dividend_info.amount is per share, so for 100 shares:
                let total_dividend = dividend_info.amount * 100.0;
                let total_hedge_cost = time_value * 100.0;

                if total_hedge_cost < total_dividend {
                    let mut signal = buy(
                        format!("Dividend Capture Target: ${:.2} (Hedge Cost: ${:.2})", total_dividend, total_hedge_cost),
                        0.15, // 15% allocation
                        Some(quote.price)
                    );
                    
                    // Add the option leg
                    signal.legs = Some(vec![
                        TradeLeg {
                            instrument_symbol: quote.symbol.clone(),
                            side: TradeSide::Buy,
                            ratio_quantity: 100,
                            position_intent: Some("buy_to_open".to_string()),
                            price: quote.price,
                            multiplier: 1.0,
                            option_type: None,
                            expiration: None,
                            strike: None,
                        },
                        TradeLeg {
                            instrument_symbol: put.contract_symbol.clone(),
                            side: TradeSide::Buy,
                            ratio_quantity: 1,
                            position_intent: Some("buy_to_open".to_string()),
                            price: put_price,
                            multiplier: 100.0,
                            option_type: Some("put".to_string()),
                            expiration: None, // TODO: Parse from symbol
                            strike: Some(put.strike),
                        }
                    ]);
                    
                    return signal;
                }
 else {
                    return hold(format!("Hedge too expensive: ${:.2} vs ${:.2} div", total_hedge_cost, total_dividend));
                }
            }
        }

        hold(format!("Monitoring ex-date: {}", dividend_info.ex_dividend_date.format("%Y-%m-%d")))
    }
}
