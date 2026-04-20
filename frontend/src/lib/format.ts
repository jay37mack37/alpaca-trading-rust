export function prettyMoney(value: number | null | undefined): string {
  return value == null ? "\u2014" : `$${value.toLocaleString(undefined, { maximumFractionDigits: 2 })}`;
}

export function prettyMoneyStrict(value: number): string {
  return `$${value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

export function prettyPct(value: number | null | undefined): string {
  return value == null ? "\u2014" : `${value >= 0 ? "+" : ""}${value.toFixed(2)}%`;
}

export function quantityDigits(assetType: string): number {
  return assetType === "equity" ? 3 : 0;
}

export function structureLabel(value: string | null | undefined): string {
  return (value ?? "single").replaceAll("_", " ");
}

export function contractLabel(item: {
  asset_type: string;
  instrument_symbol: string;
  underlying_symbol?: string | null;
  option_structure_preset?: string | null;
  option_type?: string | null;
  strike?: number | null;
  expiration?: string | null;
}): string {
  if (item.asset_type === "option_spread") {
    return [item.underlying_symbol ?? item.instrument_symbol, structureLabel(item.option_structure_preset)].join(" \u00b7 ");
  }
  if (item.asset_type !== "option") return item.instrument_symbol;
  return [
    item.instrument_symbol,
    item.option_type?.replace("_", " "),
    item.strike != null ? `$${item.strike}` : null,
    item.expiration ? new Date(item.expiration).toLocaleDateString() : null,
  ].filter(Boolean).join(" \u00b7 ");
}

export function legLabel(leg: {
  instrument_symbol: string;
  option_type?: string | null;
  strike?: number | null;
  expiration?: string | null;
}): string {
  const bits = [
    leg.option_type?.replace("_", " "),
    leg.strike != null ? `$${leg.strike}` : null,
    leg.expiration ? new Date(leg.expiration).toLocaleDateString() : null,
  ].filter(Boolean);
  return bits.length > 0 ? bits.join(" \u00b7 ") : leg.instrument_symbol;
}

export function parseSymbols(value: string): string[] {
  return value.split(",").map((item) => item.trim().toUpperCase()).filter(Boolean);
}