import { fetchWithLogging, API_BASE, devLog, devError } from './utils.js';
import { getAuthHeaders } from './auth.js';

export async function loadWatchlist() {
    const watchlistSymbols = document.getElementById('watchlist-symbols');
    const watchlistError = document.getElementById('watchlist-error');
    try {
        const response = await fetchWithLogging(`${API_BASE}/api/analytics/watchlist`, { headers: getAuthHeaders() });
        if (!response.ok) throw new Error('Failed to load watchlist');
        const data = await response.json();
        renderWatchlist(data.symbols || []);
    } catch (e) {
        if (watchlistError) { watchlistError.textContent = e.message; watchlistError.style.display = 'block'; }
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
}

export async function loadPatterns() {
    const patternCheckboxes = document.getElementById('pattern-checkboxes');
    if (patternCheckboxes && patternCheckboxes.children.length > 0) return;
    try {
        const response = await fetchWithLogging(`${API_BASE}/api/analytics/patterns`);
        if (!response.ok) return;
        const data = await response.json();
        const patterns = data.patterns || [];
        if (patternCheckboxes) {
            patternCheckboxes.innerHTML = patterns.map(p =>
                `<label class="checkbox-label"><input type="checkbox" value="${p.id}" checked> ${p.name}</label>`
            ).join('');
        }
    } catch (e) {
        console.error('Failed to load patterns', e);
    }
}

export async function runAnalysis() {
    const analysisLoading = document.getElementById('analysis-loading');
    const analysisError = document.getElementById('analysis-error');
    const analysisSymbols = document.getElementById('analysis-symbols');
    const minConfidenceInput = document.getElementById('min-confidence');
    const patternCheckboxes = document.getElementById('pattern-checkboxes');

    const symbolsStr = analysisSymbols?.value.trim() || '';
    const minConfidence = minConfidenceInput ? parseFloat(minConfidenceInput.value) || 0 : 0;
    const patterns = [];
    if (patternCheckboxes) {
        patternCheckboxes.querySelectorAll('input[type="checkbox"]:checked').forEach(cb => patterns.push(cb.value));
    }

    if (patterns.length === 0) {
        if (analysisError) {
            analysisError.textContent = 'Select at least one pattern to analyze.';
            analysisError.style.display = 'block';
        }
        return;
    }

    const body = { min_confidence: minConfidence };
    if (symbolsStr) body.symbols = symbolsStr.split(/[,\s]+/).map(s => s.toUpperCase().trim()).filter(s => s);
    if (patterns.length > 0) body.patterns = patterns;

    if (analysisLoading) analysisLoading.style.display = 'block';
    if (analysisError) analysisError.style.display = 'none';

    try {
        const response = await fetchWithLogging(`${API_BASE}/api/analytics/analyze`, {
            method: 'POST',
            headers: getAuthHeaders(),
            body: JSON.stringify(body)
        });
        if (!response.ok) throw new Error('Analysis failed');
        const data = await response.json();
        renderSignals(data.signals || []);
    } catch (e) {
        if (analysisError) { analysisError.textContent = e.message; analysisError.style.display = 'block'; }
    } finally {
        if (analysisLoading) analysisLoading.style.display = 'none';
    }
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
