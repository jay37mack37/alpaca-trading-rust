import { checkAuth, performLogout, getAuthHeaders } from './modules/auth.js';
import { fetchAccount, fetchPositions, setStatus } from './modules/ui.js';
import { fetchOrders, cancelOrder, cancelAllOrders, cancelAllOrdersInternal, cancelSelectedOrders, viewOrderDetails, updateCancelSelectedButton } from './modules/trading.js';
import { initHistory, renderHistory } from './modules/history.js';
import { initOptionsChain } from './modules/options.js';
import { loadWatchlist, addToWatchlist, fetchData, runAnalysis, loadDataSummary, loadPatterns } from './modules/analytics.js';
import { devLog, API_BASE, fetchWithLogging, openDevConsole, LOG_BUFFER, NETWORK_LOG } from './modules/utils.js';

// Expose for dev console popup window
window.LOG_BUFFER = LOG_BUFFER;
window.API_BASE = API_BASE;
window.NETWORK_LOG = NETWORK_LOG;

// Strategy log
const strategyLogEntries = [];

function addStrategyLog(message, level = 'info') {
    const timestamp = new Date().toLocaleTimeString();
    strategyLogEntries.unshift({ time: timestamp, message, level });
    if (strategyLogEntries.length > 50) strategyLogEntries.pop();
    renderStrategyLog();
}

function renderStrategyLog() {
    const logBox = document.getElementById('strategy-log-box');
    if (!logBox) return;
    if (strategyLogEntries.length === 0) {
        logBox.innerHTML = 'No log entries yet.';
        return;
    }
    logBox.innerHTML = strategyLogEntries.map(entry => `
        <div class="log-entry ${entry.level}">
            <span class="log-time">${entry.time}</span>
            <span class="log-message">${entry.message}</span>
        </div>
    `).join('');
}

async function renderStrategies() {
    try {
        const response = await fetchWithLogging(`${API_BASE}/api/strategies`, {
            headers: getAuthHeaders()
        });
        if (!response.ok) return;
        const data = await response.json();
        if (!data.success || !data.strategies) return;

        const container = document.getElementById('strategies-container');
        if (!container) return;

        container.innerHTML = data.strategies.map(s => `
            <div class="strategy-card" data-strategy-id="${s.id}">
                <h3>${s.name}</h3>
                <p class="strategy-desc">${s.description}</p>
                <div class="strategy-status">Status: <span class="status-val">${s.state}</span></div>
                <div class="strategy-actions">
                    <button class="btn-strategy-toggle" data-action="${s.state === 'Running' ? 'stop' : 'start'}">
                        ${s.state === 'Running' ? 'Stop Strategy' : 'Start Strategy'}
                    </button>
                </div>
            </div>
        `).join('');

        // Apply running styles
        container.querySelectorAll('.strategy-card').forEach(card => {
            const statusVal = card.querySelector('.status-val');
            const btn = card.querySelector('.btn-strategy-toggle');
            const state = statusVal?.textContent;
            if (state === 'Running') {
                card.classList.add('running');
                if (btn) btn.style.backgroundColor = '#c62828';
                if (statusVal) statusVal.style.color = '#4caf50';
            } else if (state === 'Error') {
                if (statusVal) statusVal.style.color = '#f44336';
            }
        });
    } catch (e) {
        devLog('STRATEGY', 'Failed to render strategies:', e);
    }
}

async function loadStrategiesStatus() {
    try {
        const response = await fetchWithLogging(`${API_BASE}/api/strategies/status`, {
            headers: getAuthHeaders()
        });
        if (!response.ok) return;
        const data = await response.json();
        if (!data.success || !data.strategies) return;

        document.querySelectorAll('.strategy-card').forEach(card => {
            const strategyId = card.dataset.strategyId;
            const strategy = data.strategies.find(s => String(s.id) === strategyId);
            if (strategy) {
                const statusVal = card.querySelector('.status-val');
                const btn = card.querySelector('.btn-strategy-toggle');
                if (statusVal) {
                    statusVal.textContent = strategy.state;
                    statusVal.style.color = strategy.state === 'Running' ? '#4caf50' : strategy.state === 'Error' ? '#f44336' : '#888';
                }
                if (btn) {
                    if (strategy.state === 'Running') {
                        btn.dataset.action = 'stop';
                        btn.textContent = 'Stop Strategy';
                        btn.style.backgroundColor = '#c62828';
                        card.classList.add('running');
                    } else {
                        btn.dataset.action = 'start';
                        btn.textContent = 'Start Strategy';
                        btn.style.backgroundColor = '#333';
                        card.classList.remove('running');
                    }
                }
            }
        });
    } catch (e) {
        devLog('STRATEGY', 'Failed to load strategy status:', e);
    }
}

document.addEventListener('DOMContentLoaded', () => {
    devLog('INIT', 'Application started');

    if (checkAuth()) {
        fetchAccount();
        fetchPositions();
        fetchOrders();
        initHistory();
        initOptionsChain();
        renderStrategies();
        loadStrategiesStatus();
    }

    // Global Event Listeners
    document.getElementById('logout-btn')?.addEventListener('click', performLogout);

    // Tab switching
    document.querySelectorAll('.tab-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            const target = btn.dataset.tab;
            document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
            btn.classList.add('active');
            document.getElementById(target)?.classList.add('active');

            if (target === 'history-tab') renderHistory();
            if (target === 'analytics-tab') {
                loadWatchlist();
                loadDataSummary();
                loadPatterns();
            }
            if (target === 'strategies-tab') {
                renderStrategies();
                loadStrategiesStatus();
            }
        });
    });

    // Order Form Class Toggle
    document.querySelectorAll('.toggle-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            const assetClass = btn.dataset.class;
            document.querySelectorAll('.toggle-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            document.getElementById('stock-fields').style.display = assetClass === 'stock' ? 'block' : 'none';
            document.getElementById('option-fields').style.display = assetClass === 'option' ? 'block' : 'none';
        });
    });

    // Order submit
    document.getElementById('order-form')?.addEventListener('submit', async (e) => {
        e.preventDefault();
        const form = e.target;
        const submitBtn = document.getElementById('submit-order');
        const assetClass = document.querySelector('.toggle-btn.active').dataset.class;

        let body = {};
        if (assetClass === 'stock') {
            body = {
                symbol: document.getElementById('symbol').value.toUpperCase(),
                side: document.getElementById('side').value,
                qty: parseFloat(document.getElementById('qty').value),
                type: document.getElementById('order-type').value,
                time_in_force: document.getElementById('time-in-force').value,
                asset_class: 'us_equity'
            };
            if (body.type === 'limit') body.limit_price = parseFloat(document.getElementById('limit-price').value);
        } else {
            body = {
                symbol: document.getElementById('option-symbol').value.toUpperCase(),
                side: document.getElementById('option-side').value.includes('buy') ? 'buy' : 'sell',
                qty: parseFloat(document.getElementById('option-qty').value),
                type: document.getElementById('option-order-type').value,
                time_in_force: document.getElementById('option-tif').value,
                asset_class: 'us_option'
                // simplified for now, usually needs the full option symbol
            };
        }

        submitBtn.disabled = true;
        try {
            const response = await fetchWithLogging(`${API_BASE}/api/orders`, {
                method: 'POST', headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${localStorage.getItem('token')}` },
                body: JSON.stringify(body)
            });
            if (!response.ok) throw new Error((response._body).error || 'Order failed');
            alert('Order placed successfully!');
            fetchOrders();
        } catch (err) { alert(`Error: ${err.message}`); }
        finally { submitBtn.disabled = false; }
    });

    // Delegate actions for dynamic buttons
    document.addEventListener('click', async (e) => {
        if (e.target.id === 'panic-btn') {
            e.target.disabled = true;
            e.target.textContent = '🚨 STOPPING...';
            try {
                // Stop all strategies
                await fetchWithLogging(`${API_BASE}/api/strategies/stop-all`, {
                    method: 'POST',
                    headers: getAuthHeaders()
                });
                // Cancel all orders (no confirm popup during panic)
                await cancelAllOrdersInternal();
                renderStrategies();
                loadStrategiesStatus();
                addStrategyLog('All strategies stopped and orders cancelled', 'info');
                devLog('PANIC', 'All strategies stopped and orders cancelled');
            } catch (err) {
                addStrategyLog('Error during panic stop: ' + err.message, 'error');
                devLog('PANIC', 'Error during panic stop:', err);
            } finally {
                e.target.disabled = false;
                e.target.textContent = '🚨 PANIC';
            }
        } else if (e.target.classList.contains('btn-cancel-order')) {
            cancelOrder(e.target.dataset.id);
        } else if (e.target.classList.contains('btn-details')) {
            viewOrderDetails(e.target.dataset.id);
        } else if (e.target.id === 'cancel-all-orders-btn') {
            cancelAllOrders();
        } else if (e.target.id === 'cancel-selected-btn') {
            cancelSelectedOrders();
        } else if (e.target.id === 'watchlist-add-btn') {
            addToWatchlist();
        } else if (e.target.id === 'fetch-data-btn') {
            fetchData(false);
        } else if (e.target.id === 'fetch-full-btn') {
            fetchData(true);
        } else if (e.target.id === 'analyze-btn') {
            runAnalysis();
        } else if (e.target.classList.contains('btn-strategy-toggle')) {
            const card = e.target.closest('.strategy-card');
            const strategyId = card?.dataset.strategyId;
            const action = e.target.dataset.action; // "start" or "stop"
            if (!strategyId) return;
            const endpoint = action === 'start'
                ? `${API_BASE}/api/strategies/${strategyId}/start`
                : `${API_BASE}/api/strategies/${strategyId}/stop`;
            e.target.disabled = true;
            try {
                const response = await fetchWithLogging(endpoint, {
                    method: 'POST',
                    headers: getAuthHeaders()
                });
                const result = await response.json();
                if (result.success) {
                    const strategyName = card?.querySelector('h3')?.textContent || `Strategy #${strategyId}`;
                    addStrategyLog(`${action === 'start' ? 'Started' : 'Stopped'} ${strategyName}`, 'info');
                    devLog('STRATEGY', result.message);
                } else {
                    addStrategyLog(`Failed: ${result.message}`, 'error');
                    alert(`Error: ${result.message}`);
                }
                renderStrategies();
                loadStrategiesStatus();
            } catch (err) {
                addStrategyLog(`Error: ${err.message}`, 'error');
                alert(`Error: ${err.message}`);
            } finally {
                e.target.disabled = false;
            }
        }
    });

    document.addEventListener('change', (e) => {
        if (e.target.classList.contains('order-checkbox') || e.target.id === 'select-all-orders') {
            if (e.target.id === 'select-all-orders') {
                document.querySelectorAll('.order-checkbox').forEach(cb => cb.checked = e.target.checked);
            }
            updateCancelSelectedButton();
        }
    });

    // Dev Console button
    document.getElementById('dev-console-btn')?.addEventListener('click', openDevConsole);

    // Auto-refresh
    setInterval(() => {
        if (localStorage.getItem('token')) {
            fetchAccount();
            fetchPositions();
            fetchOrders();
        }
    }, 30000);

    // Strategy status polling every 5 seconds
    setInterval(() => {
        if (localStorage.getItem('token')) {
            loadStrategiesStatus();
        }
    }, 5000);
});
