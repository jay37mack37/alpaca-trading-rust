use chrono::NaiveDate;

pub struct ParsedOptionContractSymbol {
    pub underlying_symbol: String,
    pub option_type: String,
    pub expiration: String,
    pub strike: f64,
}

pub fn parse_option_contract_symbol(contract_symbol: &str) -> Option<ParsedOptionContractSymbol> {
    if contract_symbol.len() < 16 {
        return None;
    }

    let root_end = contract_symbol.len().checked_sub(15)?;
    let underlying_symbol = contract_symbol.get(..root_end)?.to_uppercase();
    let expiration_part = contract_symbol.get(root_end..root_end + 6)?;
    let option_flag = contract_symbol.get(root_end + 6..root_end + 7)?;
    let strike_part = contract_symbol.get(root_end + 7..)?;

    let expiration = NaiveDate::parse_from_str(expiration_part, "%y%m%d")
        .ok()?
        .and_hms_opt(0, 0, 0)?
        .and_utc()
        .to_rfc3339();
    let option_type = match option_flag {
        "C" => "call",
        "P" => "put",
        _ => return None,
    }
    .to_string();
    let strike = strike_part.parse::<u64>().ok()? as f64 / 1000.0;

    Some(ParsedOptionContractSymbol {
        underlying_symbol,
        option_type,
        expiration,
        strike,
    })
}

/// Parse OCC option symbol to extract expiration date as "YYYY-MM-DD"
pub fn parse_expiration_from_occ(symbol: &str) -> Option<String> {
    let parsed = parse_option_contract_symbol(symbol)?;
    Some(parsed.expiration.split('T').next()?.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_call_contract_symbol() {
        let parsed =
            parse_option_contract_symbol("AAPL240119C00190000").expect("contract should parse");

        assert_eq!(parsed.underlying_symbol, "AAPL");
        assert_eq!(parsed.option_type, "call");
        assert_eq!(parsed.strike, 190.0);
        assert_eq!(parsed.expiration, "2024-01-19T00:00:00+00:00");
    }

    #[test]
    fn parses_put_contract_symbol_with_numeric_root() {
        let parsed =
            parse_option_contract_symbol("AAPL1240119P00175000").expect("contract should parse");

        assert_eq!(parsed.underlying_symbol, "AAPL1");
        assert_eq!(parsed.option_type, "put");
        assert_eq!(parsed.strike, 175.0);
        assert_eq!(parsed.expiration, "2024-01-19T00:00:00+00:00");
    }

    #[test]
    fn parses_expiration_from_occ() {
        let exp = parse_expiration_from_occ("AAPL250419C00150000").expect("should parse");
        assert_eq!(exp, "2025-04-19");
    }
}