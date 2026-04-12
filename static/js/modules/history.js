import { API_BASE, fetchWithLogging, devLog } from './utils.js';
import { getAuthHeaders } from './auth.js';
import { formatDate, formatCurrency } from './ui.js';

let currentSort = { column: 'timestamp', direction: 'desc' };

function getHistoryKey() {
    const username = localStorage.getItem('username') || 'default';
    return `trade_history_${username}`;
}

export function getHistory() {
    const data = localStorage.getItem(getHistoryKey());
    return data ? JSON.parse(data) : [];
}

export function saveHistory(history) {
    if (history.length > 1000) history = history.slice(0, 1000);
    localStorage.setItem(getHistoryKey(), JSON.stringify(history));
    renderHistory();
}

export function renderHistory() {
    const historyBody = document.getElementById('history-body');
    const historyTable = document.getElementById('history-table');
    const noHistory = document.getElementById('no-history');
    if (!historyBody) return;

    let history = getHistory();

    const symbol = document.getElementById('filter-symbol')?.value.toUpperCase() || '';
    const side = document.getElementById('filter-side')?.value || 'all';
    const start = document.getElementById('filter-start-date')?.value || '';
    const end = document.getElementById('filter-end-date')?.value || '';

    if (symbol) history = history.filter(h => h.symbol.includes(symbol));
    if (side !== 'all') history = history.filter(h => h.side === side);
    if (start) {
        const startDate = new Date(start);
        history = history.filter(h => new Date(h.timestamp) >= startDate);
    }
    if (end) {
        const endDate = new Date(end);
        endDate.setHours(23, 59, 59, 999);
        history = history.filter(h => new Date(h.timestamp) <= endDate);
    }

    history.sort((a, b) => {
        let valA = a[currentSort.column];
        let valB = b[currentSort.column];
        if (currentSort.column === 'timestamp') {
            valA = new Date(valA).getTime();
            valB = new Date(valB).getTime();
        }
        if (valA < valB) return currentSort.direction === 'asc' ? -1 : 1;
        if (valA > valB) return currentSort.direction === 'asc' ? 1 : -1;
        return 0;
    });

    if (history.length === 0) {
        if (historyTable) historyTable.style.display = 'none';
        if (noHistory) noHistory.style.display = 'block';
        return;
    }

    if (historyTable) historyTable.style.display = 'table';
    if (noHistory) noHistory.style.display = 'none';

    historyBody.innerHTML = history.map(h => `
        <tr>
            <td>${formatDate(h.timestamp)}</td>
            <td><strong>${h.symbol}</strong></td>
            <td class="${h.side.includes('buy') ? 'positive' : 'negative'}">${h.side.toUpperCase()}</td>
            <td>${h.qty}</td>
            <td>${formatCurrency(h.price)}</td>
            <td>${formatCurrency(h.amount)}</td>
            <td><span class="status-${h.status}">${h.status}</span></td>
            <td><span class="event-tag event-${h.event}">${h.event}</span></td>
        </tr>
    `).join('');
}

export function exportToCSV() {
    const history = getHistory();
    if (history.length === 0) { alert('No history to export'); return; }
    const headers = ['Timestamp', 'Symbol', 'Side', 'Qty', 'Price', 'Amount', 'Status', 'Event'];
    const rows = history.map(h => [h.timestamp, h.symbol, h.side, h.qty, h.price, h.amount, h.status, h.event]);
    const csvContent = [headers.join(','), ...rows.map(r => r.join(','))].join('\n');
    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.setAttribute('href', url);
    link.setAttribute('download', `trading_history_${new Date().toISOString().split('T')[0]}.csv`);
    link.style.visibility = 'hidden';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
}

export function logTradeEvent(order, eventType) {
    const history = getHistory();
    const status = (order.status || '').toLowerCase();
    let price = parseFloat(order.filled_avg_price || order.limit_price || 0);
    let qty = parseFloat(order.filled_qty || order.qty || 0);
    const entry = {
        id: `${order.id}_${eventType}_${new Date().getTime()}`, orderId: order.id, symbol: order.symbol,
        side: order.side, qty: order.qty, price: price, amount: price * qty, status: status, event: eventType,
        timestamp: order.filled_at || order.canceled_at || order.created_at || new Date().toISOString()
    };
    if (history.some(h => h.orderId === order.id && (h.event === eventType || h.status === status))) return;
    history.unshift(entry);
    saveHistory(history);
}

export async function backfillHistory() {
    try {
        const response = await fetchWithLogging(`${API_BASE}/api/orders?status=all`, { headers: getAuthHeaders() });
        const orders = response._body;
        if (!Array.isArray(orders)) return;
        orders.reverse().forEach(order => {
            if (order.status === 'filled') logTradeEvent(order, 'fill');
            else if (order.status === 'canceled') logTradeEvent(order, 'cancel');
            else if (order.status === 'expired') logTradeEvent(order, 'expire');
            else logTradeEvent(order, 'submit');
        });
    } catch (e) { console.error('Backfill failed', e); }
}

export function initHistory() {
    document.getElementById('filter-symbol')?.addEventListener('input', renderHistory);
    document.getElementById('filter-side')?.addEventListener('change', renderHistory);
    document.getElementById('filter-start-date')?.addEventListener('change', renderHistory);
    document.getElementById('filter-end-date')?.addEventListener('change', renderHistory);
    document.getElementById('export-csv-btn')?.addEventListener('click', exportToCSV);
    document.querySelectorAll('.sortable').forEach(th => {
        th.addEventListener('click', () => {
            const column = th.dataset.sort;
            if (currentSort.column === column) currentSort.direction = currentSort.direction === 'asc' ? 'desc' : 'asc';
            else { currentSort.column = column; currentSort.direction = 'desc'; }
            renderHistory();
        });
    });
    backfillHistory();
}
