import { fetchWithLogging, API_BASE, devLog, devError } from './utils.js';
import { getAuthHeaders } from './auth.js';
import { formatCurrency, formatDate } from './ui.js';

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

    const filterSymbol = document.getElementById('filter-symbol');
    const filterSide = document.getElementById('filter-side');
    const filterStartDate = document.getElementById('filter-start-date');
    const filterEndDate = document.getElementById('filter-end-date');

    const symbol = filterSymbol?.value.toUpperCase() || '';
    const side = filterSide?.value || 'all';
    const start = filterStartDate?.value || '';
    const end = filterEndDate?.value || '';

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

export function initHistory() {
    const filterSymbol = document.getElementById('filter-symbol');
    const filterSide = document.getElementById('filter-side');
    const filterStartDate = document.getElementById('filter-start-date');
    const filterEndDate = document.getElementById('filter-end-date');

    if (filterSymbol) filterSymbol.addEventListener('input', renderHistory);
    if (filterSide) filterSide.addEventListener('change', renderHistory);
    if (filterStartDate) filterStartDate.addEventListener('change', renderHistory);
    if (filterEndDate) filterEndDate.addEventListener('change', renderHistory);

    document.querySelectorAll('.sortable').forEach(th => {
        th.addEventListener('click', () => {
            const column = th.dataset.sort;
            if (currentSort.column === column) {
                currentSort.direction = currentSort.direction === 'asc' ? 'desc' : 'asc';
            } else {
                currentSort.column = column;
                currentSort.direction = 'desc';
            }
            renderHistory();
        });
    });
}
