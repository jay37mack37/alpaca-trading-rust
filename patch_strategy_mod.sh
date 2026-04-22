sed -i 's/pub mod vwap_reversion;/pub mod vwap_reversion;\npub mod jarrod_vwap;/g' backend/src/strategies/mod.rs
sed -i 's/m.insert(StrategyKind::VwapReversion, Box::new(VwapReversionStrategy));/m.insert(StrategyKind::VwapReversion, Box::new(VwapReversionStrategy));\n        m.insert(StrategyKind::JarrodVwap, Box::new(JarrodVwapStrategy));/g' backend/src/strategies/mod.rs
