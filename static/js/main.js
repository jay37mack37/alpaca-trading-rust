import { checkAuth, performLogout } from './modules/auth.js';
import { fetchAccount, fetchPositions, setStatus } from './modules/ui.js';
import { fetchOrders, cancelOrder, cancelAllOrders, cancelSelectedOrders, viewOrderDetails, updateCancelSelectedButton } from './modules/trading.js';
import { initHistory, renderHistory } from './modules/history.js';
import { initOptionsChain } from './modules/options.js';
import { loadWatchlist, addToWatchlist, fetchData, runAnalysis, loadDataSummary, loadPatterns } from './modules/analytics.js';
import { devLog, API_BASE, fetchWithLogging } from './modules/utils.js';

document.addEventListener('DOMContentLoaded', () => {
    devLog('INIT', 'Application started');

    if (checkAuth()) {
        fetchAccount();
        fetchPositions();
        fetchOrders();
        initHistory();
        initOptionsChain();
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
    document.addEventListener('click', (e) => {
        if (e.target.classList.contains('btn-cancel-order')) {
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

    // Auto-refresh
    setInterval(() => {
        if (localStorage.getItem('token')) {
            fetchAccount();
            fetchPositions();
            fetchOrders();
        }
    }, 30000);
});
