import type {
  BrokerSyncState,
  Candle,
  CollectResponse,
  CreateCredentialRequest,
  CreateStrategyRequest,
  DashboardResponse,
  IntelligenceLog,
  PatternAnalysisResponse,
  SetupStatusResponse,
  StrategyDetailResponse,
  StrategySummary,
  TradeRecord,
  UpdateStrategyRequest,
} from "./types";

const API_BASE = import.meta.env.VITE_API_BASE_URL ?? "";
const COMPILE_TIME_TOKEN = (import.meta.env.VITE_API_TOKEN ?? "").trim();
const STORAGE_KEY = "autostonks_api_token";
const PLACEHOLDER_TOKENS = ["your_api_token_here"];

function getRuntimeToken(): string {
  let stored = "";
  try {
    stored = localStorage.getItem(STORAGE_KEY)?.trim() ?? "";
  } catch {
    // localStorage unavailable (e.g. test environment)
  }
  if (stored.length > 0 && !PLACEHOLDER_TOKENS.includes(stored)) {
    return stored;
  }
  if (COMPILE_TIME_TOKEN.length > 0 && !PLACEHOLDER_TOKENS.includes(COMPILE_TIME_TOKEN)) {
    return COMPILE_TIME_TOKEN;
  }
  return "";
}

export function apiTokenConfigured(): boolean {
  return getRuntimeToken().length > 0;
}

export function setApiToken(token: string) {
  try {
    localStorage.setItem(STORAGE_KEY, token.trim());
  } catch {
    // localStorage unavailable
  }
}

export function clearApiToken() {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    // localStorage unavailable
  }
}

interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const token = getRuntimeToken();
  if (!token) {
    throw new Error(
      "VITE_API_TOKEN is not set. Copy the token printed by the backend on first start into frontend/.env as VITE_API_TOKEN=<token>.",
    );
  }

  const response = await fetch(`${API_BASE}${path}`, {
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
      ...(init?.headers ?? {}),
    },
    ...init,
  });

  const text = await response.text();
  let body: ApiResponse<T>;
  try {
    body = JSON.parse(text) as ApiResponse<T>;
  } catch (err) {
    throw new Error(text || `Request failed with ${response.status}`);
  }

  if (!body.success || !response.ok) {
    throw new Error(body.error || `Request failed with ${response.status}`);
  }

  if (body.data === undefined) {
    return {} as T;
  }

  return body.data;
}

export async function fetchSetupStatus(): Promise<SetupStatusResponse> {
  const response = await fetch(`${API_BASE}/api/setup/status`);
  const body: ApiResponse<SetupStatusResponse> = await response.json();
  if (!body.success) throw new Error(body.error || "Failed to fetch setup status");
  return body.data!;
}

export async function writeEnvToken(apiToken: string): Promise<{ written: boolean; message: string }> {
  const response = await fetch(`${API_BASE}/api/setup/env`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ api_token: apiToken }),
  });
  const body: ApiResponse<{ written: boolean; message: string }> = await response.json();
  if (!body.success || !response.ok) {
    throw new Error(body.error || `Request failed with ${response.status}`);
  }
  return body.data!;
}

export const api = {
  streamUrl(symbol: string, provider: string, strategyId?: string) {
    const token = getRuntimeToken();
    const params = new URLSearchParams({ symbol, provider });
    if (strategyId) params.set("strategy_id", strategyId);
    if (token) params.set("token", token);
    return `${API_BASE}/api/stream?${params.toString()}`;
  },
  dashboard(symbol: string, provider: string) {
    const params = new URLSearchParams({ symbol, provider });
    return request<DashboardResponse>(`/api/dashboard?${params.toString()}`);
  },
  collectNow() {
    return request<CollectResponse>("/api/collect", { method: "POST" });
  },
  createCredential(payload: CreateCredentialRequest) {
    return request<any>("/api/credentials", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  },
  createStrategy(payload: CreateStrategyRequest) {
    return request<StrategySummary>("/api/strategies", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  },
  strategyDetail(strategyId: string) {
    return request<StrategyDetailResponse>(`/api/strategies/${strategyId}`);
  },
  updateStrategy(strategyId: string, payload: UpdateStrategyRequest) {
    return request<StrategySummary>(`/api/strategies/${strategyId}`, {
      method: "PATCH",
      body: JSON.stringify(payload),
    });
  },
  fetchStrategyLogs(strategyId: string) {
    return request<IntelligenceLog[]>(`/api/strategies/${strategyId}/logs`);
  },
  runStrategy(strategyId: string, symbol?: string) {
    const suffix = symbol ? `?symbol=${encodeURIComponent(symbol)}` : "";
    return request<TradeRecord | null>(`/api/strategies/${strategyId}/run${suffix}`, {
      method: "POST",
    });
  },
  startStrategy(strategyId: string) {
    return request<void>(`/api/strategies/${strategyId}/start`, {
      method: "POST",
    });
  },
  stopStrategy(strategyId: string) {
    return request<void>(`/api/strategies/${strategyId}/stop`, {
      method: "POST",
    });
  },
  flattenPosition(strategyId: string, symbol: string) {
    return request<void>(`/api/strategies/${strategyId}/positions/${symbol}/flatten`, {
      method: "POST",
    });
  },
  syncStrategy(strategyId: string) {
    return request<BrokerSyncState>(`/api/strategies/${strategyId}/alpaca-sync`, {
      method: "POST",
    });
  },
  delete(path: string) {
    return request<void>(path, {
      method: "DELETE",
    });
  },
  panic() {
    return request<void>("/api/strategies/panic", {
      method: "POST",
    });
  },
  liquidateAllBrokerPositions() {
    return request<void>("/api/broker/liquidate-all", {
      method: "POST",
    });
  },
  runPatternAnalysis(symbols: string, provider?: string, minConfidence?: number) {
    const params = new URLSearchParams();
    if (symbols) params.set("symbols", symbols);
    if (provider) params.set("provider", provider);
    if (minConfidence != null) params.set("min_confidence", String(minConfidence));
    return request<PatternAnalysisResponse>(`/api/analytics/patterns?${params.toString()}`);
  },
  marketCandles(symbol: string, provider?: string, range?: string, interval?: string) {
    const params = new URLSearchParams();
    if (provider) params.set("provider", provider);
    if (range) params.set("range", range);
    if (interval) params.set("interval", interval);
    return request<Candle[]>(`/api/market/candles/${symbol}?${params.toString()}`);
  },
};