import { checkAuth, performLogout } from './modules/auth.js';
import { fetchAccount, fetchPositions } from './modules/ui.js';
import { fetchOrders, cancelOrder, viewOrderDetails, updateCancelSelectedButton } from './modules/trading.js';
import { devLog, openDevConsole } from './modules/utils.js';
import { initHistory, renderHistory } from './modules/history.js';
import { loadWatchlist, loadPatterns, runAnalysis } from './modules/analytics.js';

// Global app initialization
document.addEventListener('DOMContentLoaded', () => {
    devLog('INIT', 'Application started via ES Modules');

    if (checkAuth()) {
        fetchAccount();
        fetchPositions();
        fetchOrders();
        initHistory();
    }

    // Attach global event listeners
    document.getElementById('logout-btn')?.addEventListener('click', performLogout);
    document.getElementById('dev-console-btn')?.addEventListener('click', openDevConsole);

    // Delegate actions
    document.addEventListener('click', (e) => {
        if (e.target.classList.contains('btn-cancel-order')) {
            cancelOrder(e.target.dataset.id);
        } else if (e.target.classList.contains('btn-details')) {
            viewOrderDetails(e.target.dataset.id);
        } else if (e.target.id === 'analyze-btn') {
            runAnalysis();
        }
    });

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
                loadPatterns();
            }
        });
    });

    document.addEventListener('change', (e) => {
        if (e.target.classList.contains('order-checkbox')) {
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
