import { API_BASE, fetchWithLogging, devLog } from './utils.js';
import { getAuthHeaders } from './auth.js';

export async function loadWatchlist() {
    const watchlistSymbols = document.getElementById('watchlist-symbols');
    const watchlistError = document.getElementById('watchlist-error');
    const watchlistLoading = document.getElementById('watchlist-loading');
    if (watchlistLoading) watchlistLoading.style.display = 'block';
    try {
        const response = await fetchWithLogging(`${API_BASE}/api/analytics/watchlist`, { headers: getAuthHeaders() });
        if (!response.ok) throw new Error('Failed to load watchlist');
        const data = await response.json();
        renderWatchlist(data.symbols || []);
    } catch (e) {
        if (watchlistError) { watchlistError.textContent = e.message; watchlistError.style.display = 'block'; }
    } finally {
        if (watchlistLoading) watchlistLoading.style.display = 'none';
    }
}

function renderWatchlist(symbols) {
    const watchlistSymbols = document.getElementById('watchlist-symbols');
    if (!watchlistSymbols) return;
    if (symbols.length === 0) {
        watchlistSymbols.innerHTML = '<span style="color: #888;">No symbols in watchlist. Add some above.</span>';
        return;
    }
    watchlistSymbols.innerHTML = symbols.map(sym => `
        <div class="watchlist-tag">
            ${sym}
            <span class="remove-tag" data-symbol="${sym}">&times;</span>
        </div>
    `).join('');

    document.querySelectorAll('.remove-tag').forEach(btn => {
        btn.onclick = () => removeFromWatchlist(btn.dataset.symbol);
    });
}

export async function addToWatchlist() {
    const input = document.getElementById('watchlist-add-input');
    const symbols = input.value.split(/[,\s]+/).map(s => s.trim().toUpperCase()).filter(s => s);
    if (symbols.length === 0) return;
    try {
        await fetchWithLogging(`${API_BASE}/api/analytics/watchlist`, {
            method: 'POST', headers: getAuthHeaders(), body: JSON.stringify({ symbols })
        });
        input.value = '';
        loadWatchlist();
    } catch (e) { alert('Failed to add symbols'); }
}

export async function removeFromWatchlist(symbol) {
    try {
        await fetchWithLogging(`${API_BASE}/api/analytics/watchlist/${symbol}`, {
            method: 'DELETE', headers: getAuthHeaders()
        });
        loadWatchlist();
    } catch (e) { alert('Failed to remove symbol'); }
}

export async function loadPatterns() {
    const patternCheckboxes = document.getElementById('pattern-checkboxes');
    try {
        const response = await fetchWithLogging(`${API_BASE}/api/analytics/patterns`);
        const data = await response.json();
        const patterns = data.patterns || [];
        if (patternCheckboxes) {
            patternCheckboxes.innerHTML = patterns.map(p =>
                `<label class="checkbox-label"><input type="checkbox" value="${p.id}" checked> ${p.name}</label>`
            ).join('');
        }
    } catch (e) { console.error('Failed to load patterns', e); }
}

export async function fetchData(fullRefresh) {
    const dataLoading = document.getElementById('data-loading');
    const dataError = document.getElementById('data-error');
    const symbolsStr = document.getElementById('fetch-symbols-input')?.value || '';
    const source = document.getElementById('fetch-source-select')?.value || 'auto';
    const timeframes = Array.from(document.querySelectorAll('#fetch-timeframes input:checked')).map(cb => cb.value);

    const body = { source, timeframes, full_refresh: fullRefresh };
    if (symbolsStr) body.symbols = symbolsStr.split(/[,\s]+/).map(s => s.toUpperCase().trim()).filter(s => s);

    if (dataLoading) dataLoading.style.display = 'block';
    if (dataError) dataError.style.display = 'none';

    try {
        const response = await fetchWithLogging(`${API_BASE}/api/analytics/fetch-data`, {
            method: 'POST', headers: getAuthHeaders(), body: JSON.stringify(body)
        });
        if (!response.ok) throw new Error('Data fetch failed');
        loadDataSummary();
    } catch (e) { if (dataError) { dataError.textContent = e.message; dataError.style.display = 'block'; } }
    finally { if (dataLoading) dataLoading.style.display = 'none'; }
}

export async function loadDataSummary() {
    const container = document.getElementById('data-summary-container');
    try {
        const response = await fetchWithLogging(`${API_BASE}/api/analytics/data-summary`, { headers: getAuthHeaders() });
        const data = await response.json();
        if (container) {
            container.innerHTML = `<div class="data-summary">
                <p>Cache contains <strong>${data.symbol_count}</strong> symbols across <strong>${data.timeframe_count}</strong> timeframes.</p>
                <p>Last updated: ${data.last_update ? new Date(data.last_update).toLocaleString() : 'Never'}</p>
            </div>`;
        }
    } catch (e) { console.error('Failed to load summary', e); }
}

export async function runAnalysis() {
    const analysisLoading = document.getElementById('analysis-loading');
    const analysisError = document.getElementById('analysis-error');
    const symbolsStr = document.getElementById('analysis-symbols')?.value || '';
    const minConfidence = parseFloat(document.getElementById('min-confidence')?.value || '0');
    const storeSignals = document.getElementById('store-signals-checkbox')?.checked || false;
    const updateFirst = document.getElementById('update-data-checkbox')?.checked || false;
    const patterns = Array.from(document.querySelectorAll('#pattern-checkboxes input:checked')).map(cb => cb.value);

    if (patterns.length === 0) {
        if (analysisError) { analysisError.textContent = 'Select at least one pattern.'; analysisError.style.display = 'block'; }
        return;
    }

    const body = { min_confidence: minConfidence, store_signals: storeSignals, update_data_first: updateFirst, patterns };
    if (symbolsStr) body.symbols = symbolsStr.split(/[,\s]+/).map(s => s.toUpperCase().trim()).filter(s => s);

    if (analysisLoading) analysisLoading.style.display = 'block';
    if (analysisError) analysisError.style.display = 'none';

    try {
        const response = await fetchWithLogging(`${API_BASE}/api/analytics/analyze`, {
            method: 'POST', headers: getAuthHeaders(), body: JSON.stringify(body)
        });
        if (!response.ok) throw new Error('Analysis failed');
        const data = await response.json();
        renderSignals(data.signals || []);
    } catch (e) { if (analysisError) { analysisError.textContent = e.message; analysisError.style.display = 'block'; } }
    finally { if (analysisLoading) analysisLoading.style.display = 'none'; }
}

function renderSignals(signals) {
    const signalsBody = document.getElementById('signals-body');
    const signalsTable = document.getElementById('signals-table');
    const signalsNoData = document.getElementById('signals-no-data');
    if (!signalsBody || !signalsTable || !signalsNoData) return;

    if (signals.length === 0) {
        signalsTable.style.display = 'none';
        signalsNoData.style.display = 'block';
        return;
    }

    signalsTable.style.display = 'table';
    signalsNoData.style.display = 'none';
    signalsBody.innerHTML = signals.map(s => `
        <tr>
            <td>${s.timestamp ? new Date(s.timestamp).toLocaleString() : '-'}</td>
            <td style="font-weight:700;">${s.symbol}</td>
            <td>${s.pattern}</td>
            <td class="direction-${s.direction}">${s.direction}</td>
            <td>${Math.round(s.confidence * 100)}%</td>
            <td style="font-size:0.8rem;">${JSON.stringify(s.details)}</td>
        </tr>
    `).join('');
}
