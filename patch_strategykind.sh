sed -i 's/VwapReversion,/VwapReversion,\n    JarrodVwap,/g' backend/src/models/mod.rs
sed -i 's/Self::VwapReversion => "vwap_reversion",/Self::VwapReversion => "vwap_reversion",\n            Self::JarrodVwap => "jarrod_vwap",/g' backend/src/models/mod.rs
