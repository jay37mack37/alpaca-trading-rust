import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

describe('api.streamUrl', () => {
  let api: any;

  beforeEach(async () => {
    vi.resetModules();
  });

  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it('should build a basic stream URL with symbol and provider', async () => {
    vi.stubEnv('VITE_API_TOKEN', '');
    api = (await import('./api')).api;
    const url = api.streamUrl('AAPL', 'yahoo');
    const urlObj = new URL(url, 'http://localhost');
    expect(urlObj.pathname).toBe('/api/stream');
    expect(urlObj.searchParams.get('symbol')).toBe('AAPL');
    expect(urlObj.searchParams.get('provider')).toBe('yahoo');
  });

  it('should include strategy_id when provided', async () => {
    vi.stubEnv('VITE_API_TOKEN', '');
    api = (await import('./api')).api;
    const url = api.streamUrl('AAPL', 'yahoo', 'strat-123');
    const urlObj = new URL(url, 'http://localhost');
    expect(urlObj.pathname).toBe('/api/stream');
    expect(urlObj.searchParams.get('symbol')).toBe('AAPL');
    expect(urlObj.searchParams.get('provider')).toBe('yahoo');
    expect(urlObj.searchParams.get('strategy_id')).toBe('strat-123');
  });

  it('should URL-encode special characters in parameters', async () => {
    vi.stubEnv('VITE_API_TOKEN', '');
    api = (await import('./api')).api;
    const url = api.streamUrl('BRK.B', 'yahoo', 'strat/456');
    const urlObj = new URL(url, 'http://localhost');
    expect(urlObj.searchParams.get('symbol')).toBe('BRK.B');
    expect(urlObj.searchParams.get('provider')).toBe('yahoo');
    expect(urlObj.searchParams.get('strategy_id')).toBe('strat/456');
  });

  it('should include API_TOKEN when VITE_API_TOKEN is set', async () => {
    vi.stubEnv('VITE_API_TOKEN', 'test-token-123');
    api = (await import('./api')).api;
    const url = api.streamUrl('AAPL', 'yahoo');
    const urlObj = new URL(url, 'http://localhost');
    expect(urlObj.searchParams.get('token')).toBe('test-token-123');
  });
});
