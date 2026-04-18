import type {
  BrokerSyncState,
  CollectResponse,
  CreateCredentialRequest,
  CreateStrategyRequest,
  DashboardResponse,
  StrategyDetailResponse,
  StrategySummary,
  TradeRecord,
  UpdateStrategyRequest,
} from "./types";

const API_BASE = import.meta.env.VITE_API_BASE_URL ?? "http://127.0.0.1:8080";
const API_TOKEN = (import.meta.env.VITE_API_TOKEN ?? "").trim();

export const apiTokenConfigured = API_TOKEN.length > 0;

interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  if (!API_TOKEN) {
    throw new Error(
      "VITE_API_TOKEN is not set. Copy the token printed by the backend on first start into frontend/.env as VITE_API_TOKEN=<token>.",
    );
  }

  const response = await fetch(`${API_BASE}${path}`, {
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${API_TOKEN}`,
      ...(init?.headers ?? {}),
    },
    ...init,
  });

  let body: ApiResponse<T>;
  try {
    body = (await response.json()) as ApiResponse<T>;
  } catch (err) {
    const text = await response.text();
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

export const api = {
  streamUrl(symbol: string, provider: string, strategyId?: string) {
    // EventSource cannot attach custom headers, so the backend also accepts a
    // `?token=` query-string fallback on the auth middleware.
    const params = new URLSearchParams({ symbol, provider });
    if (strategyId) params.set("strategy_id", strategyId);
    if (API_TOKEN) params.set("token", API_TOKEN);
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
  syncStrategy(strategyId: string) {
    return request<BrokerSyncState>(`/api/strategies/${strategyId}/alpaca-sync`, {
      method: "POST",
    });
  },
  panic() {
    return request<void>("/api/panic", {
      method: "POST",
    });
  },
};
