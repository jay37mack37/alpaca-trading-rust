import { fetchWithLogging, API_BASE, devLog, devError } from './utils.js';
import { getAuthHeaders } from './auth.js';

export function formatCurrency(value) {
    const num = parseFloat(value);
    if (isNaN(num)) return '-';
    return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(num);
}

export function formatPercent(value) {
    const num = parseFloat(value);
    if (isNaN(num)) return '-';
    const sign = num >= 0 ? '+' : '';
    return `${sign}${(num * 100).toFixed(2)}%`;
}

export function formatDate(dateStr) {
    if (!dateStr) return '-';
    const date = new Date(dateStr);
    return date.toLocaleString();
}

export function setStatus(status, isError = false) {
    const statusText = document.getElementById('status-text');
    const statusDot = document.querySelector('.status-dot');
    if (!statusText || !statusDot) return;
    statusText.textContent = status;
    statusDot.classList.remove('connected', 'error');
    if (isError) statusDot.classList.add('error');
    else statusDot.classList.add('connected');
}

export async function fetchAccount() {
    const accountLoading = document.getElementById('account-loading');
    const accountInfo = document.getElementById('account-info');
    const accountError = document.getElementById('account-error');
    if (!accountLoading) return;

    accountLoading.style.display = 'block';
    accountInfo.style.display = 'none';
    accountError.style.display = 'none';

    try {
        const response = await fetchWithLogging(`${API_BASE}/api/account`, { headers: getAuthHeaders() });
        const data = response._body;
        if (!response.ok) throw new Error(data.error || data.message || 'Failed to fetch account');

        accountLoading.style.display = 'none';
        accountInfo.style.display = 'grid';

        document.getElementById('account-number').textContent = data.account_number || '-';
        const statusEl = document.getElementById('account-status');
        statusEl.textContent = data.status || '-';
        statusEl.className = `value ${data.status === 'ACTIVE' ? 'positive' : ''}`;
        document.getElementById('buying-power').textContent = formatCurrency(data.buying_power);
        document.getElementById('portfolio-value').textContent = formatCurrency(data.portfolio_value);
        document.getElementById('cash').textContent = formatCurrency(data.cash);
        document.getElementById('equity').textContent = formatCurrency(data.equity);

        setStatus('Connected');
    } catch (err) {
        accountLoading.style.display = 'none';
        accountError.style.display = 'block';
        accountError.textContent = `Error: ${err.message}`;
        setStatus('Connection Error', true);
    }
}

export async function fetchPositions() {
    const positionsLoading = document.getElementById('positions-loading');
    const positionsTable = document.getElementById('positions-table');
    const positionsBody = document.getElementById('positions-body');
    const noPositions = document.getElementById('no-positions');
    const positionsError = document.getElementById('positions-error');
    if (!positionsLoading) return;

    positionsLoading.style.display = 'block';
    positionsTable.style.display = 'none';
    noPositions.style.display = 'none';
    positionsError.style.display = 'none';

    try {
        const response = await fetchWithLogging(`${API_BASE}/api/positions`, { headers: getAuthHeaders() });
        const data = response._body;
        if (!response.ok) throw new Error(data.error || data.message || 'Failed to fetch positions');

        positionsLoading.style.display = 'none';
        if (!Array.isArray(data) || data.length === 0) {
            noPositions.style.display = 'block';
            return;
        }

        positionsTable.style.display = 'table';
        positionsBody.innerHTML = data.map(pos => `
            <tr>
                <td><strong>${pos.symbol}</strong></td>
                <td>${pos.qty}</td>
                <td>${formatCurrency(pos.avg_entry_price)}</td>
                <td>${formatCurrency(pos.current_price)}</td>
                <td>${formatCurrency(pos.market_value)}</td>
                <td class="${parseFloat(pos.unrealized_plpc) >= 0 ? 'positive' : 'negative'}">
                    ${formatCurrency(pos.unrealized_pl)} (${formatPercent(pos.unrealized_plpc)})
                </td>
            </tr>
        `).join('');
    } catch (err) {
        positionsLoading.style.display = 'none';
        positionsError.style.display = 'block';
        positionsError.textContent = `Error: ${err.message}`;
    }
}
