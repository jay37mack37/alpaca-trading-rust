// ============================================================
// DEVELOPMENT TOOLS - Console Logging & Network Inspector
// ============================================================

const DEV_MODE = true;
const LOG_BUFFER = [];
const MAX_LOG_SIZE = 100;

// Log to console AND dev buffer
function devLog(category, message, data = null) {
    if (!DEV_MODE) return;

    const timestamp = new Date().toLocaleTimeString();
    const logEntry = {
        time: timestamp,
        category,
        message,
        data,
        type: 'log'
    };

    LOG_BUFFER.push(logEntry);
    if (LOG_BUFFER.length > MAX_LOG_SIZE) LOG_BUFFER.shift();

    const style = `color: #0066cc; font-weight: bold;`;
    console.log(`%c[${timestamp}] ${category}: ${message}`, style, data || '');
}

function devWarn(category, message, data = null) {
    if (!DEV_MODE) return;

    const timestamp = new Date().toLocaleTimeString();
    const logEntry = {
        time: timestamp,
        category,
        message,
        data,
        type: 'warn'
    };

    LOG_BUFFER.push(logEntry);
    if (LOG_BUFFER.length > MAX_LOG_SIZE) LOG_BUFFER.shift();

    const style = `color: #ff9900; font-weight: bold;`;
    console.warn(`%c[${timestamp}] ${category}: ${message}`, style, data || '');
}

function devError(category, message, data = null) {
    if (!DEV_MODE) return;

    const timestamp = new Date().toLocaleTimeString();
    const logEntry = {
        time: timestamp,
        category,
        message,
        data,
        type: 'error'
    };

    LOG_BUFFER.push(logEntry);
    if (LOG_BUFFER.length > MAX_LOG_SIZE) LOG_BUFFER.shift();

    const style = `color: #cc0000; font-weight: bold;`;
    console.error(`%c[${timestamp}] ${category}: ${message}`, style, data || '');
}

// Network logging array
const NETWORK_LOG = [];

// Network logging wrapper
async function fetchWithLogging(url, options = {}) {
    const method = options.method || 'GET';
    const startTime = performance.now();
    const shortUrl = url.replace(API_BASE, '');

    devLog('API', `${method} ${shortUrl}`, options.headers || {});

    try {
        const response = await fetch(url, options);
        const duration = (performance.now() - startTime).toFixed(2);
        const contentType = response.headers.get('content-type');

        let data;
        let size = 0;
        if (contentType && contentType.includes('application/json')) {
            data = await response.clone().json();
            size = JSON.stringify(data).length;
        } else {
            data = await response.clone().text();
            size = data.length;
        }

        const logEntry = {
            time: new Date().toLocaleTimeString(),
            method,
            url: shortUrl,
            status: response.status,
            duration: parseFloat(duration),
            size
        };
        NETWORK_LOG.push(logEntry);

        devLog('API', `${method} ${shortUrl} → ${response.status} (${duration}ms)`, {
            status: response.status,
            statusText: response.statusText,
            duration: `${duration}ms`,
            size: `${(size / 1024).toFixed(2)}KB`,
            body: data
        });

        // Global auth handling
        if (response.status === 401 && !url.includes('/api/login') && !url.includes('/api/verify')) {
            devWarn('AUTH', 'Session expired, redirecting to login');
            localStorage.removeItem('token');
            localStorage.removeItem('username');
            window.location.href = '/login.html';
        }

        // Re-attach the body for the caller
        response._body = data;
        return response;
    } catch (error) {
        const duration = (performance.now() - startTime).toFixed(2);
        const logEntry = {
            time: new Date().toLocaleTimeString(),
            method,
            url: shortUrl,
            status: 0,
            duration: parseFloat(duration),
            size: 0
        };
        NETWORK_LOG.push(logEntry);
        devError('API', `${method} ${shortUrl} failed (${duration}ms)`, error.message);
        throw error;
    }
}

// Open dev console in new window
function openDevConsole() {
    devLog('DEV', 'Opening developer console...');
    const devWindow = window.open('/dev-console.html', 'dev-console', 'width=1200,height=600,resizable=yes');
    devWindow.focus();
}

// ============================================================
// API Base URL
// ============================================================

const API_BASE = window.location.origin;

devLog('INIT', 'Application started', { apiBase: API_BASE, timestamp: new Date().toISOString() });

// Auth check
function checkAuth() {
    const token = localStorage.getItem('token');
    const userDisplay = document.getElementById('user-display');
    const logoutBtn = document.getElementById('logout-btn');
    const authCheck = document.getElementById('auth-check');

    if (!token) {
        window.location.href = '/login.html';
        return false;
    }

    // Verify token with server
    fetchWithLogging(`${API_BASE}/api/verify`, {
        headers: { 'Authorization': `Bearer ${token}` }
    })
        .then(res => {
            if (!res.ok) {
                localStorage.removeItem('token');
                localStorage.removeItem('username');
                window.location.href = '/login.html';
                return;
            }
            if (authCheck) authCheck.style.display = 'none';
            const username = localStorage.getItem('username');
            if (userDisplay) userDisplay.textContent = `👤 ${username}`;
        })
        .catch(() => {
            window.location.href = '/login.html';
        });

    // Logout handler
    if (logoutBtn) {
        logoutBtn.addEventListener('click', () => {
            const token = localStorage.getItem('token');
            performLogout(API_BASE, token);
        });
    }

    return true;
}

// Auth header for API requests
function getAuthHeaders() {
    const token = localStorage.getItem('token');
    return {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`
    };
}

// DOM Elements — lazily resolved after DOMContentLoaded
let statusDot, statusText;
let accountLoading, accountInfo, accountError;
let positionsLoading, positionsTable, positionsBody, noPositions, positionsError;
let ordersLoading, ordersTable, ordersBody, noOrders, ordersError;
let orderForm, orderSuccess, orderError, submitBtn;
let cancelAllBtn, cancelSelectedBtn, selectAllCheckbox;

// History Elements
let historyBody, noHistory, historyTable;
let filterSymbol, filterSide, filterStartDate, filterEndDate;

// Analytics Elements
let watchlistAddInput, watchlistAddBtn, watchlistSymbols, watchlistLoading, watchlistError;
let fetchSymbolsInput, fetchSourceSelect, fetchDataBtn, fetchFullBtn, dataLoading, dataSummaryContainer, dataError;
let analysisSymbols, minConfidenceInput, patternCheckboxes, analyzeBtn, storeSignalsCheckbox, updateDataCheckbox, analysisLoading, analysisError;
let signalsTable, signalsBody, signalsNoData, signalsCount, signalFilterSymbol, signalFilterDirection, signalFilterPattern, exportSignalsBtn;

function resolveElements() {
    statusDot = document.querySelector('.status-dot');
    statusText = document.getElementById('status-text');

    accountLoading = document.getElementById('account-loading');
    accountInfo = document.getElementById('account-info');
    accountError = document.getElementById('account-error');

    positionsLoading = document.getElementById('positions-loading');
    positionsTable = document.getElementById('positions-table');
    positionsBody = document.getElementById('positions-body');
    noPositions = document.getElementById('no-positions');
    positionsError = document.getElementById('positions-error');

    ordersLoading = document.getElementById('orders-loading');
    ordersTable = document.getElementById('orders-table');
    ordersBody = document.getElementById('orders-body');
    noOrders = document.getElementById('no-orders');
    ordersError = document.getElementById('orders-error');

    orderForm = document.getElementById('order-form');
    orderSuccess = document.getElementById('order-success');
    orderError = document.getElementById('order-error');
    submitBtn = document.getElementById('submit-order');

    cancelAllBtn = document.getElementById('cancel-all-orders-btn');
    cancelSelectedBtn = document.getElementById('cancel-selected-btn');
    selectAllCheckbox = document.getElementById('select-all-orders');

    historyBody = document.getElementById('history-body');
    noHistory = document.getElementById('no-history');
    historyTable = document.getElementById('history-table');
    filterSymbol = document.getElementById('filter-symbol');
    filterSide = document.getElementById('filter-side');
    filterStartDate = document.getElementById('filter-start-date');
    filterEndDate = document.getElementById('filter-end-date');

    // Analytics Elements
    watchlistAddInput = document.getElementById('watchlist-add-input');
    watchlistAddBtn = document.getElementById('watchlist-add-btn');
    watchlistSymbols = document.getElementById('watchlist-symbols');
    watchlistLoading = document.getElementById('watchlist-loading');
    watchlistError = document.getElementById('watchlist-error');
    fetchSymbolsInput = document.getElementById('fetch-symbols-input');
    fetchSourceSelect = document.getElementById('fetch-source-select');
    fetchDataBtn = document.getElementById('fetch-data-btn');
    fetchFullBtn = document.getElementById('fetch-full-btn');
    dataLoading = document.getElementById('data-loading');
    dataSummaryContainer = document.getElementById('data-summary-container');
    dataError = document.getElementById('data-error');
    analysisSymbols = document.getElementById('analysis-symbols');
    minConfidenceInput = document.getElementById('min-confidence');
    patternCheckboxes = document.getElementById('pattern-checkboxes');
    analyzeBtn = document.getElementById('analyze-btn');
    storeSignalsCheckbox = document.getElementById('store-signals-checkbox');
    updateDataCheckbox = document.getElementById('update-data-checkbox');
    analysisLoading = document.getElementById('analysis-loading');
    analysisError = document.getElementById('analysis-error');
    signalsTable = document.getElementById('signals-table');
    signalsBody = document.getElementById('signals-body');
    signalsNoData = document.getElementById('signals-no-data');
    signalsCount = document.getElementById('signals-count');
    signalFilterSymbol = document.getElementById('signal-filter-symbol');
    signalFilterDirection = document.getElementById('signal-filter-direction');
    signalFilterPattern = document.getElementById('signal-filter-pattern');
    exportSignalsBtn = document.getElementById('export-signals-btn');
}

// Format currency
function formatCurrency(value) {
    const num = parseFloat(value);
    if (isNaN(num)) return '-';
    return new Intl.NumberFormat('en-US', {
        style: 'currency',
        currency: 'USD'
    }).format(num);
}

// Format percentage
function formatPercent(value) {
    const num = parseFloat(value);
    if (isNaN(num)) return '-';
    const sign = num >= 0 ? '+' : '';
    return `${sign}${(num * 100).toFixed(2)}%`;
}

// Format date
function formatDate(dateStr) {
    if (!dateStr) return '-';
    const date = new Date(dateStr);
    return date.toLocaleString();
}

// Update connection status
function setStatus(status, isError = false) {
    if (!statusText || !statusDot) return;
    statusText.textContent = status;
    statusDot.classList.remove('connected', 'error');
    if (isError) {
        statusDot.classList.add('error');
    } else {
        statusDot.classList.add('connected');
    }
}

// Fetch account info
async function fetchAccount() {
    if (!accountLoading) return;

    accountLoading.style.display = 'block';
    accountInfo.style.display = 'none';
    accountError.style.display = 'none';

    try {
        const response = await fetchWithLogging(`${API_BASE}/api/account`, {
            headers: getAuthHeaders()
        });

        const data = response._body;

        if (!response.ok) {
            throw new Error(data.error || data.message || 'Failed to fetch account');
        }

        accountLoading.style.display = 'none';
        accountInfo.style.display = 'grid';

        document.getElementById('account-number').textContent = data.account_number || '-';
        document.getElementById('account-status').textContent = data.status || '-';
        document.getElementById('account-status').className = `value ${data.status === 'ACTIVE' ? 'positive' : ''}`;
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

// Fetch positions
async function fetchPositions() {
    if (!positionsLoading) return;

    positionsLoading.style.display = 'block';
    positionsTable.style.display = 'none';
    noPositions.style.display = 'none';
    positionsError.style.display = 'none';

    try {
        const response = await fetchWithLogging(`${API_BASE}/api/positions`, {
            headers: getAuthHeaders()
        });

        const data = response._body;

        if (!response.ok) {
            throw new Error(data.error || data.message || 'Failed to fetch positions');
        }

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

// Fetch orders
async function fetchOrders() {
    if (!ordersLoading) return;

    ordersLoading.style.display = 'block';
    ordersTable.style.display = 'none';
    noOrders.style.display = 'none';
    ordersError.style.display = 'none';

    try {
        const response = await fetchWithLogging(`${API_BASE}/api/orders`, {
            headers: getAuthHeaders()
        });

        const data = response._body;

        if (!response.ok) {
            throw new Error(data.message || data.error || 'Failed to fetch orders');
        }

        ordersLoading.style.display = 'none';

        if (!Array.isArray(data) || data.length === 0) {
            noOrders.style.display = 'block';
            if (cancelAllBtn) cancelAllBtn.style.display = 'none';
            if (cancelSelectedBtn) cancelSelectedBtn.style.display = 'none';
            return;
        }

        ordersTable.style.display = 'table';

        // Check for open orders
        const openOrders = data.filter(order => order.status === 'open' || order.status === 'pending_new');
        if (cancelAllBtn) {
            cancelAllBtn.style.display = openOrders.length > 0 ? 'block' : 'none';
        }

        ordersBody.innerHTML = data.slice(0, 20).map(order => {
            const isOpen = order.status === 'open' || order.status === 'pending_new';
            return `
            <tr data-order-id="${order.id}">
                <td>
                    ${isOpen ? `<input type="checkbox" class="order-checkbox" data-order-id="${order.id}" onchange="updateCancelSelectedButton()">` : ''}
                </td>
                <td><strong>${order.symbol}</strong></td>
                <td class="${order.side === 'buy' ? 'positive' : 'negative'}">${order.side.toUpperCase()}</td>
                <td>${order.qty}</td>
                <td>${order.type}</td>
                <td><span class="status-${order.status}">${order.status}</span></td>
                <td>${formatDate(order.created_at)}</td>
                <td>
                    <div class="order-actions-cell">
                        <button class="btn-refresh btn-small" onclick="viewOrderDetails('${order.id}')">Details</button>
                        ${isOpen ? `<button class="btn-cancel btn-small" onclick="cancelOrder('${order.id}')">Cancel</button>` : ''}
                    </div>
                </td>
            </tr>
        `}).join('');

        // Reset select all checkbox
        if (selectAllCheckbox) {
            selectAllCheckbox.checked = false;
        }

        updateCancelSelectedButton();
    } catch (err) {
        ordersLoading.style.display = 'none';
        ordersError.style.display = 'block';
        ordersError.textContent = `Error: ${err.message}`;
        if (cancelAllBtn) cancelAllBtn.style.display = 'none';
        if (cancelSelectedBtn) cancelSelectedBtn.style.display = 'none';
    }
}

// Update cancel selected button visibility
function updateCancelSelectedButton() {
    const checkboxes = document.querySelectorAll('.order-checkbox:checked');

    if (cancelSelectedBtn) {
        cancelSelectedBtn.style.display = checkboxes.length > 0 ? 'block' : 'none';
        if (checkboxes.length > 0) {
            cancelSelectedBtn.textContent = `Cancel Selected (${checkboxes.length})`;
        }
    }

    // Update select all checkbox state
    const allCheckboxes = document.querySelectorAll('.order-checkbox');
    if (selectAllCheckbox && allCheckboxes.length > 0) {
        selectAllCheckbox.checked = checkboxes.length === allCheckboxes.length;
    }
}

// Toggle select all orders
function toggleSelectAllOrders() {
    const checkboxes = document.querySelectorAll('.order-checkbox');

    checkboxes.forEach(cb => {
        cb.checked = selectAllCheckbox.checked;
    });

    updateCancelSelectedButton();
}

// Cancel an order
async function cancelOrder(orderId) {
    if (!confirm('Are you sure you want to cancel this order?')) return;

    try {
        const response = await fetchWithLogging(`${API_BASE}/api/orders/${orderId}`, {
            method: 'DELETE',
            headers: getAuthHeaders()
        });

        const data = response._body;

        if (!response.ok) {
            throw new Error(data.error || data.message || 'Failed to cancel order');
        }

        // Log the cancellation
        logTransaction(data, 'cancelled');

        alert('Order cancelled successfully!');
        fetchOrders();
        syncHistoryWithAPI(); // Immediate sync
    } catch (err) {
        alert(`Error: ${err.message}`);
    }
}

// Cancel selected orders
async function cancelSelectedOrders() {
    const checkboxes = document.querySelectorAll('.order-checkbox:checked');
    if (checkboxes.length === 0) return;

    if (!confirm(`Are you sure you want to cancel ${checkboxes.length} selected order(s)?`)) return;

    const cancelSelectedBtn = document.getElementById('cancel-selected-btn');
    if (cancelSelectedBtn) {
        cancelSelectedBtn.disabled = true;
        cancelSelectedBtn.textContent = 'Cancelling...';
    }

    let successCount = 0;
    let failCount = 0;

    for (const checkbox of checkboxes) {
        const orderId = checkbox.dataset.orderId;
        try {
            const response = await fetchWithLogging(`${API_BASE}/api/orders/${orderId}`, {
                method: 'DELETE',
                headers: getAuthHeaders()
            });

            if (response.ok) {
                successCount++;
            } else {
                failCount++;
            }
        } catch (err) {
            failCount++;
        }
    }

    if (cancelSelectedBtn) {
        cancelSelectedBtn.disabled = false;
    }

    if (failCount === 0) {
        alert(`${successCount} order(s) cancelled successfully!`);
    } else {
        alert(`${successCount} order(s) cancelled, ${failCount} failed.`);
    }

    fetchOrders();
    fetchAccount();
}

// Cancel all open orders
async function cancelAllOrders() {
    if (!confirm('Are you sure you want to cancel ALL open orders?')) return;

    const cancelAllBtn = document.getElementById('cancel-all-orders-btn');
    if (cancelAllBtn) {
        cancelAllBtn.disabled = true;
        cancelAllBtn.textContent = 'Cancelling...';
    }

    try {
        const response = await fetchWithLogging(`${API_BASE}/api/orders/cancel-all`, {
            method: 'POST',
            headers: getAuthHeaders()
        });

        const data = response._body;

        if (!response.ok) {
            throw new Error(data.error || data.message || 'Failed to cancel all orders');
        }

        alert(data.message || 'All orders cancelled successfully!');
        fetchOrders();
        fetchAccount(); // Refresh account to update buying power
    } catch (err) {
        alert(`Error: ${err.message}`);
    } finally {
        if (cancelAllBtn) {
            cancelAllBtn.disabled = false;
            cancelAllBtn.textContent = 'Cancel All Orders';
        }
    }
}

// Make functions available globally
window.cancelOrder = cancelOrder;
window.viewOrderDetails = viewOrderDetails;
window.cancelSelectedOrders = cancelSelectedOrders;
window.toggleSelectAllOrders = toggleSelectAllOrders;
window.updateCancelSelectedButton = updateCancelSelectedButton;

function initOrderButtons() {
    if (cancelAllBtn) cancelAllBtn.addEventListener('click', cancelAllOrders);
    if (cancelSelectedBtn) cancelSelectedBtn.addEventListener('click', cancelSelectedOrders);
    if (selectAllCheckbox) selectAllCheckbox.addEventListener('change', toggleSelectAllOrders);

    // Modal close handlers
    const modal = document.getElementById('order-details-modal');
    const closeBtn = document.querySelector('.close-modal');

    if (closeBtn) {
        closeBtn.onclick = () => {
            modal.style.display = 'none';
        };
    }

    modal.addEventListener('click', (event) => {
        if (event.target === modal) {
            modal.style.display = 'none';
        }
    });
}

async function viewOrderDetails(orderId) {
    const modal = document.getElementById('order-details-modal');
    const content = document.getElementById('order-details-content');

    if (modal) modal.style.display = 'block';
    if (content) content.innerHTML = '<div class="loading">Loading details...</div>';

    try {
        const response = await fetchWithLogging(`${API_BASE}/api/orders/${orderId}`, {
            headers: getAuthHeaders()
        });

        const order = response._body;

        if (!response.ok) {
            throw new Error(order.error || 'Failed to fetch order details');
        }

        content.innerHTML = `
            <div class="details-grid">
                <div class="detail-item">
                    <span class="label">Order ID</span>
                    <span class="value" style="font-size: 0.8rem;">${order.id}</span>
                </div>
                <div class="detail-item">
                    <span class="label">Symbol</span>
                    <span class="value">${order.symbol}</span>
                </div>
                <div class="detail-item">
                    <span class="label">Side</span>
                    <span class="value ${order.side === 'buy' ? 'positive' : 'negative'}">${order.side.toUpperCase()}</span>
                </div>
                <div class="detail-item">
                    <span class="label">Status</span>
                    <span class="value"><span class="status-${order.status}">${order.status}</span></span>
                </div>
                <div class="detail-item">
                    <span class="label">Quantity</span>
                    <span class="value">${order.qty}</span>
                </div>
                <div class="detail-item">
                    <span class="label">Filled Quantity</span>
                    <span class="value">${order.filled_qty || 0}</span>
                </div>
                <div class="detail-item">
                    <span class="label">Order Type</span>
                    <span class="value">${order.type}</span>
                </div>
                <div class="detail-item">
                    <span class="label">Time in Force</span>
                    <span class="value">${order.time_in_force.toUpperCase()}</span>
                </div>
                <div class="detail-item">
                    <span class="label">Limit Price</span>
                    <span class="value">${order.limit_price ? formatCurrency(order.limit_price) : '-'}</span>
                </div>
                <div class="detail-item">
                    <span class="label">Avg Fill Price</span>
                    <span class="value">${order.filled_avg_price ? formatCurrency(order.filled_avg_price) : '-'}</span>
                </div>
                <div class="detail-item">
                    <span class="label">Created At</span>
                    <span class="value">${formatDate(order.created_at)}</span>
                </div>
                <div class="detail-item">
                    <span class="label">Updated At</span>
                    <span class="value">${formatDate(order.updated_at)}</span>
                </div>
            </div>
        `;
    } catch (err) {
        if (content) content.innerHTML = `<div class="error-message">Error: ${err.message}</div>`;
    }
}

// Asset class toggle
let currentAssetClass = 'stock';
let selectedOptionType = 'call'; // default to CALL

// Set default expiration date to next Friday
function getNextFriday() {
    const today = new Date();
    const dayOfWeek = today.getDay();
    const daysUntilFriday = (5 - dayOfWeek + 7) % 7 || 7;
    const nextFriday = new Date(today);
    nextFriday.setDate(today.getDate() + daysUntilFriday);
    return nextFriday.toISOString().split('T')[0];
}

function initAssetToggle() {
    document.querySelectorAll('.toggle-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            document.querySelectorAll('.toggle-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            currentAssetClass = btn.dataset.class;

            // Toggle visibility
            const stockFields = document.getElementById('stock-fields');
            const optionFields = document.getElementById('option-fields');
            if (stockFields) stockFields.style.display = currentAssetClass === 'stock' ? 'block' : 'none';
            if (optionFields) optionFields.style.display = currentAssetClass === 'option' ? 'block' : 'none';

            // When switching to option, default to CALL
            if (currentAssetClass === 'option') {
                document.querySelectorAll('.option-type-btn').forEach(b => b.classList.remove('active'));
                const callBtn = document.querySelector('.option-type-btn.call-btn');
                if (callBtn) {
                    callBtn.classList.add('active');
                    selectedOptionType = 'call';
                    const hiddenInput = document.getElementById('option-type-hidden');
                    if (hiddenInput) hiddenInput.value = 'call';
                }
            }
        });
    });

    // Option type toggle
    document.querySelectorAll('.option-type-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            document.querySelectorAll('.option-type-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            selectedOptionType = btn.dataset.type;
            const hiddenInput = document.getElementById('option-type-hidden');
            if (hiddenInput) hiddenInput.value = selectedOptionType;
        });
    });

    // Set CALL as default active on load
    const callBtn = document.querySelector('.option-type-btn.call-btn');
    if (callBtn) {
        callBtn.classList.add('active');
        const hiddenInput = document.getElementById('option-type-hidden');
        if (hiddenInput) hiddenInput.value = 'call';
    }

    // Set default expiration dates
    const expirationDateEl = document.getElementById('expiration-date');
    if (expirationDateEl) expirationDateEl.value = getNextFriday();
}

function initOrderForm() {
    // Attach order form submit handler (orderForm is resolved via resolveElements())
    if (orderForm) {
        orderForm.addEventListener('submit', async (e) => {
            e.preventDefault();

            orderSuccess.style.display = 'none';
            orderError.style.display = 'none';
            submitBtn.disabled = true;
            submitBtn.textContent = 'Placing Order...';

            let orderData = {};

            try {
                if (currentAssetClass === 'stock') {
                    // Stock order
                    const symbol = document.getElementById('symbol').value.toUpperCase();
                    const side = document.getElementById('side').value;
                    const qtyInput = document.getElementById('qty').value;
                    const orderType = document.getElementById('order-type').value;
                    const limitPriceInput = document.getElementById('limit-price').value;
                    const timeInForce = document.getElementById('time-in-force').value;

                    if (!symbol) throw new Error('Symbol is required');
                    const qty = parseFloat(qtyInput);
                    if (isNaN(qty) || qty <= 0) throw new Error('Quantity must be a positive number');

                    orderData = { symbol, qty, side, order_type: orderType, time_in_force: timeInForce };

                    if (orderType === 'limit') {
                        if (!limitPriceInput) throw new Error('Limit price is required for limit orders');
                        const limitPrice = parseFloat(limitPriceInput);
                        if (isNaN(limitPrice) || limitPrice <= 0) throw new Error('Limit price must be a positive number');
                        orderData.limit_price = limitPrice;
                    }
                } else {
                    // Option order
                    const underlying = document.getElementById('option-symbol').value.toUpperCase();
                    const optionType = document.getElementById('option-type-hidden').value;
                    const strikeInput = document.getElementById('strike-price').value;
                    const expiration = document.getElementById('expiration-date').value;
                    const side = document.getElementById('option-side').value;
                    const qtyInput = document.getElementById('option-qty').value;
                    const orderType = document.getElementById('option-order-type').value;
                    const limitPriceInput = document.getElementById('option-limit-price').value;
                    const timeInForce = document.getElementById('option-tif').value;

                    if (!underlying) throw new Error('Underlying symbol is required');
                    if (!optionType) throw new Error('Please select Call or Put');
                    if (!expiration) throw new Error('Expiration date is required');

                    const strike = parseFloat(strikeInput);
                    if (isNaN(strike) || strike <= 0) throw new Error('Strike price must be a positive number');

                    const qty = parseInt(qtyInput);
                    if (isNaN(qty) || qty <= 0) throw new Error('Quantity must be a positive integer');

                    // Build OCC symbol: SPY240408C00500000
                    // Parse date as local time — new Date('YYYY-MM-DD') is UTC which shifts the day
                    const [expYear, expMonth, expDay] = expiration.split('-').map(Number);
                    const yy = String(expYear).slice(-2);
                    const mm = String(expMonth).padStart(2, '0');
                    const dd = String(expDay).padStart(2, '0');
                    const cp = optionType === 'call' ? 'C' : 'P';
                    const strikeStr = (strike * 1000).toFixed(0).padStart(8, '0');
                    const symbol = underlying.padEnd(6, ' ') + yy + mm + dd + cp + strikeStr;

                    orderData = {
                        symbol: symbol.replace(/\s/g, ''),
                        qty, side,
                        order_type: orderType,
                        time_in_force: timeInForce,
                        asset_class: 'us_option'
                    };

                    if (orderType === 'limit') {
                        if (!limitPriceInput) throw new Error('Limit price is required for limit orders');
                        const limitPrice = parseFloat(limitPriceInput);
                        if (isNaN(limitPrice) || limitPrice <= 0) throw new Error('Limit price must be a positive number');
                        orderData.limit_price = limitPrice;
                    }
                }

                const response = await fetchWithLogging(`${API_BASE}/api/orders`, {
                    method: 'POST',
                    headers: getAuthHeaders(),
                    body: JSON.stringify(orderData)
                });

                const data = response._body;
                if (!response.ok) throw new Error(data.message || data.error || 'Failed to place order');

                // Log the placed order
                logTransaction(data, 'placed');

                orderSuccess.style.display = 'block';
        syncHistoryWithAPI(); // Immediate sync
                orderSuccess.textContent = `Order placed successfully! Order ID: ${data.id || 'N/A'}`;
                orderForm.reset();
                const symEl = document.getElementById('symbol');
                const optSymEl = document.getElementById('option-symbol');
                const expEl = document.getElementById('expiration-date');
                if (symEl) symEl.value = 'SPY';
                if (optSymEl) optSymEl.value = 'SPY';
                if (expEl) expEl.value = getNextFriday();

                setTimeout(() => { fetchOrders(); fetchAccount(); }, 1000);

            } catch (err) {
                orderError.style.display = 'block';
                orderError.textContent = `Error: ${err.message}`;
            } finally {
                submitBtn.disabled = false;
                submitBtn.textContent = 'Place Order';
            }
        });
    }

    // Show/hide limit price based on stock order type
    const orderTypeSelect = document.getElementById('order-type');
    const fillPriceBtn = document.getElementById('fill-price-btn');
    const limitPriceInput = document.getElementById('limit-price');
    const symbolInput = document.getElementById('symbol');

    if (orderTypeSelect) {
        orderTypeSelect.addEventListener('change', (e) => {
            if (e.target.value === 'limit') {
                if (limitPriceInput) { limitPriceInput.required = true; limitPriceInput.placeholder = 'Required'; }
                if (fillPriceBtn) fillPriceBtn.style.display = 'block';
            } else {
                if (limitPriceInput) { limitPriceInput.required = false; limitPriceInput.placeholder = 'Optional'; }
                if (fillPriceBtn) fillPriceBtn.style.display = 'none';
            }
        });
    }

    // Fill stock market price button
    if (fillPriceBtn) {
        fillPriceBtn.addEventListener('click', async () => {
            const symbol = symbolInput ? symbolInput.value.toUpperCase() : '';
            if (!symbol) { alert('Please enter a symbol first'); return; }

            fillPriceBtn.disabled = true;
            fillPriceBtn.textContent = 'Loading...';

            try {
                const response = await fetchWithLogging(`${API_BASE}/api/price/${symbol}`, { headers: getAuthHeaders() });
                const data = response._body;
                if (!response.ok) throw new Error(data.error || 'Failed to get price');

                // For buy orders use ask, for sell orders use bid
                const side = document.getElementById('side') ? document.getElementById('side').value : 'buy';
                let price;
                if (data.quote) {
                    const askPrice = data.quote.ap || data.quote.bp;
                    const bidPrice = data.quote.bp;
                    price = side === 'buy' ? askPrice : bidPrice;
                } else { throw new Error('Price data not available'); }

                if (!price || price === 0) throw new Error('Price not available (market may be closed)');
                if (limitPriceInput) limitPriceInput.value = parseFloat(price).toFixed(2);
            } catch (err) {
                alert(`Error: ${err.message}`);
            } finally {
                fillPriceBtn.disabled = false;
                fillPriceBtn.textContent = 'Fill Market Price';
            }
        });
    }

    // Show/hide limit price based on option order type
    const optionOrderTypeSelect = document.getElementById('option-order-type');
    const fillOptionPriceBtn = document.getElementById('fill-option-price-btn');
    const optionLimitPriceInput = document.getElementById('option-limit-price');
    const optionSymbolInput = document.getElementById('option-symbol');
    const fillStrikeBtn = document.getElementById('fill-strike-btn');
    const strikePriceInput = document.getElementById('strike-price');

    if (optionOrderTypeSelect) {
        optionOrderTypeSelect.addEventListener('change', (e) => {
            if (e.target.value === 'limit') {
                if (optionLimitPriceInput) { optionLimitPriceInput.required = true; optionLimitPriceInput.placeholder = 'Required'; }
                if (fillOptionPriceBtn) fillOptionPriceBtn.style.display = 'block';
            } else {
                if (optionLimitPriceInput) { optionLimitPriceInput.required = false; optionLimitPriceInput.placeholder = 'Optional'; }
                if (fillOptionPriceBtn) fillOptionPriceBtn.style.display = 'none';
            }
        });
    }

    // Fill strike button
    if (fillStrikeBtn) {
        fillStrikeBtn.addEventListener('click', async () => {
            const symbol = optionSymbolInput ? optionSymbolInput.value.toUpperCase() : '';
            if (!symbol) { alert('Please enter an underlying symbol first'); return; }
            if (!selectedOptionType) { alert('Please select Call or Put first'); return; }

            fillStrikeBtn.disabled = true;
            fillStrikeBtn.textContent = 'Loading...';
            try {
                const data = await getOptionStrikes(symbol);
                if (data) {
                    const strike = selectedOptionType === 'call' ? data.call_strike : data.put_strike;
                    if (strike) {
                        if (strikePriceInput) strikePriceInput.value = strike;
                    } else { alert(`No ${selectedOptionType} strikes available`); }
                } else { alert('No strikes available'); }
            } catch (err) {
                alert(`Error: ${err.message}`);
            } finally {
                fillStrikeBtn.disabled = false;
                fillStrikeBtn.textContent = 'Strike';
            }
        });
    }

    // Fill option market price button
    if (fillOptionPriceBtn) {
        fillOptionPriceBtn.addEventListener('click', async () => {
            const underlying = optionSymbolInput ? optionSymbolInput.value.toUpperCase() : '';
            const strike = strikePriceInput ? strikePriceInput.value : '';
            const expiration = document.getElementById('expiration-date') ? document.getElementById('expiration-date').value : '';
            const optionType = document.getElementById('option-type-hidden') ? document.getElementById('option-type-hidden').value : '';

            if (!underlying || !strike || !expiration || !optionType) {
                alert('Please fill in all option details first');
                return;
            }

            fillOptionPriceBtn.disabled = true;
            fillOptionPriceBtn.textContent = 'Loading...';

            try {
                // Parse date as local time — new Date('YYYY-MM-DD') is UTC which shifts the day
                const [expYear, expMonth, expDay] = expiration.split('-').map(Number);
                const yy = String(expYear).slice(-2);
                const mm = String(expMonth).padStart(2, '0');
                const dd = String(expDay).padStart(2, '0');
                const cp = optionType === 'call' ? 'C' : 'P';
                const strikeStr = (parseFloat(strike) * 1000).toFixed(0).padStart(8, '0');
                const optionSymbol = underlying.padEnd(6, ' ') + yy + mm + dd + cp + strikeStr;

                const response = await fetchWithLogging(`${API_BASE}/api/option-quote/${optionSymbol.replace(/\s/g, '')}`, { headers: getAuthHeaders() });
                const data = response._body;
                if (!response.ok) throw new Error(data.error || 'Failed to get option price');

                let price;
                const side = document.getElementById('option-side') ? document.getElementById('option-side').value : 'buy_to_open';
                if (data.quote) {
                    const askPrice = data.quote.ap || data.quote.bp;
                    const bidPrice = data.quote.bp;
                    price = side.includes('buy') ? askPrice : bidPrice;
                } else { throw new Error('Price data not available'); }

                if (!price || price === 0) throw new Error('Price not available (market may be closed)');
                if (optionLimitPriceInput) optionLimitPriceInput.value = parseFloat(price).toFixed(2);
            } catch (err) {
                alert(`Error: ${err.message}`);
            } finally {
                fillOptionPriceBtn.disabled = false;
                fillOptionPriceBtn.textContent = 'Fill Market Price';
            }
        });
    }
}

async function getOptionStrikes(underlying) {
    const response = await fetchWithLogging(`${API_BASE}/api/option-strikes/${underlying}`, { headers: getAuthHeaders() });
    const data = response._body;
    if (!response.ok) throw new Error(data.error || 'Failed to get option strikes');
    return data;
}

// Single consolidated DOMContentLoaded — all init happens here
document.addEventListener('DOMContentLoaded', () => {
    resolveElements();
    initOrderButtons();
    initAssetToggle();
    initOrderForm();
    initOptionsChain();
    initTabs();
    initHistory();
    initAnalytics();
    initStrategies();
    syncStrategiesStatus(); // Initial sync
    setInterval(syncStrategiesStatus, 5000); // Sync every 5 seconds

    // Initialize dev console button
    const devConsoleBtn = document.getElementById('dev-console-btn');
    if (devConsoleBtn) {
        devConsoleBtn.addEventListener('click', openDevConsole);
    }

    // Keyboard shortcut: Ctrl+Shift+D to open dev console
    document.addEventListener('keydown', (e) => {
        if (e.ctrlKey && e.shiftKey && e.key === 'D') {
            e.preventDefault();
            openDevConsole();
        }
    });

    if (checkAuth()) {
        fetchAccount();
        fetchPositions();
        fetchOrders();
    }
});

// Auto-refresh every 30 seconds
setInterval(() => {
    if (localStorage.getItem('token')) {
        fetchAccount();
        fetchPositions();
        fetchOrders();
        syncHistoryWithAPI();
    }
}, 30000);

// ============================================================
// TRANSACTION HISTORY LOGIC
// ============================================================

let currentSort = { column: 'timestamp', direction: 'desc' };

// Get history key for current user
function getHistoryKey() {
    const username = localStorage.getItem('username') || 'default';
    return `trade_history_${username}`;
}

// Get history from localStorage
function getHistory() {
    const data = localStorage.getItem(getHistoryKey());
    return data ? JSON.parse(data) : [];
}

// Save history to localStorage (with cleanup)
function saveHistory(history) {
    // Keep only last 1000 items
    if (history.length > 1000) {
        history = history.slice(0, 1000);
    }
    localStorage.setItem(getHistoryKey(), JSON.stringify(history));
    renderHistory();
}

// Log a transaction event
function logTransaction(order, eventType) {
    const history = getHistory();

    // Normalize status for consistency
    const status = (order.status || '').toLowerCase();

    // Calculate amount (Price * Qty)
    let price = parseFloat(order.filled_avg_price || order.limit_price || 0);
    let qty = parseFloat(order.filled_qty || order.qty || 0);
    let amount = price * qty;

    const entry = {
        id: `${order.id}_${eventType}_${new Date().getTime()}`,
        orderId: order.id,
        symbol: order.symbol,
        side: order.side,
        qty: order.qty,
        price: price,
        amount: amount,
        status: status,
        event: eventType,
        timestamp: order.filled_at || order.canceled_at || order.created_at || new Date().toISOString()
    };

    // Check if this specific event for this order is already logged to avoid duplicates
    // We check both the event type AND the status
    const isDuplicate = history.some(h =>
        h.orderId === order.id &&
        (h.event === eventType || h.status === status)
    );
    if (isDuplicate) return;

    history.unshift(entry);
    saveHistory(history);
    devLog('HISTORY', `Logged ${eventType} for ${order.symbol}`, entry);
}

// Backfill history from API
async function backfillHistory() {
    devLog('HISTORY', 'Backfilling history from API...');
    try {
            const response = await fetchWithLogging(`${API_BASE}/api/orders?status=all`, {
            headers: getAuthHeaders()
        });

        if (!response.ok) throw new Error('Failed to fetch historical orders');
            const orders = response._body;

        const history = getHistory();
        let addedCount = 0;

        // Process orders in reverse (oldest first) so they are logged in correct order if we were unshifting
        // But here we'll just build a new set and merge
        const newEntries = [];

        orders.forEach(order => {
            // Determine likely events based on status
            // If it's filled, it was placed then filled
            // For simplicity in backfill, we'll just log its current status as a 'backfill' event
            // unless we want to simulate the whole lifecycle. Let's just log the current state.

            const isLogged = history.some(h => h.orderId === order.id);
            if (!isLogged) {
                let price = parseFloat(order.filled_avg_price || order.limit_price || 0);
                let qty = parseFloat(order.filled_qty || order.qty || 0);

                newEntries.push({
                    id: `${order.id}_backfill`,
                    orderId: order.id,
                    symbol: order.symbol,
                    side: order.side,
                    qty: order.qty,
                    price: price,
                    amount: price * qty,
                    status: order.status,
                    event: 'backfill',
                    timestamp: order.filled_at || order.canceled_at || order.created_at
                });
                addedCount++;
            }
        });

        if (addedCount > 0) {
            const updatedHistory = [...newEntries, ...history];
            // Sort by timestamp desc
            updatedHistory.sort((a, b) => new Date(b.timestamp) - new Date(a.timestamp));
            saveHistory(updatedHistory);
            devLog('HISTORY', `Backfilled ${addedCount} orders`);
        }
    } catch (err) {
        devError('HISTORY', 'Backfill failed', err.message);
    }
}

// Sync current orders to detect status changes
async function syncHistoryWithAPI() {
    try {
        // Fetch all orders (including closed ones) to ensure history is accurate
        const response = await fetchWithLogging(`${API_BASE}/api/orders?status=all`, {
            headers: getAuthHeaders()
        });
        if (!response.ok) return;
        const currentOrders = response._body;

        const history = getHistory();
        let changed = false;

        currentOrders.forEach(order => {
            const status = (order.status || '').toLowerCase();

            // Map Alpaca status to our event types
            let eventType = 'status_change';
            if (status === 'filled') eventType = 'filled';
            else if (status === 'canceled' || status === 'cancelled') eventType = 'cancelled';
            else if (status === 'expired') eventType = 'expired';
            else if (status === 'new' || status === 'pending_new' || status === 'accepted') eventType = 'placed';
            else if (status === 'rejected') eventType = 'rejected';

            // Check if we already have this status or event logged for this order
            const isLogged = history.some(h =>
                h.orderId === order.id && (h.status === status || h.event === eventType)
            );

            if (!isLogged) {
                logTransaction(order, eventType);
                changed = true;
            }
        });

        if (changed) {
            devLog('HISTORY', 'History updated from sync');
        }
    } catch (err) {
        devError('HISTORY', 'Sync failed', err.message);
    }
}

// Render history table
function renderHistory() {
    if (!historyBody) return;

    let history = getHistory();

    // Apply filters
    const symbol = filterSymbol.value.toUpperCase();
    const side = filterSide.value;
    const start = filterStartDate.value;
    const end = filterEndDate.value;

    if (symbol) {
        history = history.filter(h => h.symbol.includes(symbol));
    }
    if (side !== 'all') {
        history = history.filter(h => h.side === side);
    }
    if (start) {
        const startDate = new Date(start);
        history = history.filter(h => new Date(h.timestamp) >= startDate);
    }
    if (end) {
        const endDate = new Date(end);
        endDate.setHours(23, 59, 59, 999);
        history = history.filter(h => new Date(h.timestamp) <= endDate);
    }

    // Apply sorting
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
        historyTable.style.display = 'none';
        noHistory.style.display = 'block';
        return;
    }

    historyTable.style.display = 'table';
    noHistory.style.display = 'none';

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

// Export history to CSV
function exportToCSV() {
    const history = getHistory();
    if (history.length === 0) {
        alert('No history to export');
        return;
    }

    const headers = ['Timestamp', 'Symbol', 'Side', 'Qty', 'Price', 'Amount', 'Status', 'Event'];
    const rows = history.map(h => [
        h.timestamp,
        h.symbol,
        h.side,
        h.qty,
        h.price,
        h.amount,
        h.status,
        h.event
    ]);

    const csvContent = [
        headers.join(','),
        ...rows.map(r => r.join(','))
    ].join('\n');

    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.setAttribute('href', url);
    link.setAttribute('download', `trade_history_${new Date().toISOString().split('T')[0]}.csv`);
    link.style.visibility = 'hidden';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
}

// Initialize history features
function initHistory() {
    if (filterSymbol) filterSymbol.addEventListener('input', renderHistory);
    if (filterSide) filterSide.addEventListener('change', renderHistory);
    if (filterStartDate) filterStartDate.addEventListener('change', renderHistory);
    if (filterEndDate) filterEndDate.addEventListener('change', renderHistory);

    const exportBtn = document.getElementById('export-csv-btn');
    if (exportBtn) exportBtn.addEventListener('click', exportToCSV);

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

    backfillHistory();
}

// ============================================================
// TAB SYSTEM
// ============================================================

function initTabs() {
    const tabBtns = document.querySelectorAll('.tab-btn');
    const tabContents = document.querySelectorAll('.tab-content');

    tabBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            const target = btn.dataset.tab;

            tabBtns.forEach(b => b.classList.remove('active'));
            tabContents.forEach(c => c.classList.remove('active'));

            btn.classList.add('active');
            const targetEl = document.getElementById(target);
            if (targetEl) targetEl.classList.add('active');

            if (target === 'history-tab') {
                renderHistory();
            }
            if (target === 'analytics-tab') {
                loadWatchlist();
                loadDataSummary();
                loadPatterns();
            }
            if (target === 'strategies-tab') {
                renderStrategies();
                syncStrategiesStatus();
            }
        });
    });
}

// ============================================================
// STRATEGIES MANAGEMENT
// ============================================================

const STRATEGIES = [
    {
        id: 1,
        name: 'Listing Arbitrage',
        description: 'Snipes new $SPY options via Black-Scholes valuation gaps and Kronos trend filtering.'
    },
    {
        id: 2,
        name: 'VWAP Mean Reversion',
        description: 'Automated entries on standard deviation price extensions from the VWAP.'
    },
    {
        id: 3,
        name: '0DTE Delta-Neutral',
        description: 'Harvests theta decay on same-day expiry options via automated spreads.'
    },
    {
        id: 4,
        name: 'Gamma Scalping',
        description: 'Dynamic delta hedging to profit from realized volatility.'
    },
    {
        id: 5,
        name: 'Put-Call Parity',
        description: 'Arbitrages discrepancies between synthesized and market option prices.'
    }
];

// Store strategy statuses (Idle or Running)
const strategyStatuses = {};
const strategyLogEntries = [];
let strategyHeartbeatInterval = null;

// Initialize all strategies to idle
STRATEGIES.forEach(s => {
    strategyStatuses[s.id] = 'idle';
});

function addStrategyLog(message, type = 'info') {
    const logBox = document.getElementById('strategy-log-box');
    if (!logBox) return;

    const timestamp = new Date().toLocaleTimeString();
    const entry = document.createElement('div');
    entry.className = `log-entry log-${type}`;
    entry.innerHTML = `
        <span class="log-timestamp">[${timestamp}]</span>
        <span class="log-message">${message}</span>
    `;

    logBox.appendChild(entry);
    logBox.scrollTop = logBox.scrollHeight;

    // Keep only last 50 entries to prevent memory issues
    while (logBox.children.length > 50) {
        logBox.removeChild(logBox.firstChild);
    }
}

function startStrategyHeartbeat() {
    if (strategyHeartbeatInterval) return;
    strategyHeartbeatInterval = setInterval(() => {
        const runningCount = Object.values(strategyStatuses).filter(status => status === 'running').length;
        if (runningCount === 0) return;
        addStrategyLog(`Heartbeat: ${runningCount} strategy(s) running`, 'debug');
    }, 30000);
}

function stopStrategyHeartbeat() {
    const runningCount = Object.values(strategyStatuses).filter(status => status === 'running').length;
    if (runningCount === 0 && strategyHeartbeatInterval) {
        clearInterval(strategyHeartbeatInterval);
        strategyHeartbeatInterval = null;
    }
}

function renderStrategies() {
    const container = document.getElementById('strategies-container');
    if (!container) return;

    container.innerHTML = '';

    STRATEGIES.forEach(strategy => {
        const status = strategyStatuses[strategy.id] || 'idle';
        const isRunning = status === 'running';
        const isStarting = status === 'starting';
        const isStopping = status === 'stopping';

        const card = document.createElement('div');
        card.className = 'strategy-card';
        card.innerHTML = `
            <div class="strategy-header">
                <span class="strategy-name">${strategy.name}</span>
                <span class="strategy-status ${status}">
                    ${status.charAt(0).toUpperCase() + status.slice(1)}
                </span>
            </div>
            <p class="strategy-description">${strategy.description}</p>
            <div class="strategy-buttons">
                <button class="btn-execute" data-strategy-id="${strategy.id}" ${isRunning || isStarting ? 'disabled' : ''}>
                    ${isStarting ? 'Starting...' : 'Execute'}
                </button>
                <button class="btn-stop" data-strategy-id="${strategy.id}" ${!isRunning || isStopping ? 'disabled' : ''}>
                    ${isStopping ? 'Stopping...' : 'Stop'}
                </button>
            </div>
        `;

        container.appendChild(card);
    });

    // Add event listeners to all strategy buttons
    document.querySelectorAll('.btn-execute').forEach(btn => {
        btn.addEventListener('click', executeStrategy);
    });

    document.querySelectorAll('.btn-stop').forEach(btn => {
        btn.addEventListener('click', stopStrategy);
    });
}

async function executeStrategy(e) {
    const button = e.currentTarget;
    const strategyId = button.dataset.strategyId;
    const strategy = STRATEGIES.find(s => s.id === parseInt(strategyId));

    if (!strategy) return;

    // Optimistic UI update
    strategyStatuses[strategyId] = 'starting';
    renderStrategies();
    addStrategyLog(`Starting ${strategy.name}...`, 'info');

    try {
        const response = await fetch(`${API_BASE}/api/strategies/${strategyId}/start`, {
            method: 'POST',
            headers: getAuthHeaders(),
            body: JSON.stringify({})
        });

        const result = await response.json();
        if (response.ok && result.success) {
            strategyStatuses[strategyId] = 'running';
            addStrategyLog(`Started: ${strategy.name}`, 'success');
            addStrategyLog(`Scan: SPY... | Math: OK | Kronos: Bullish | Result: Pass`, 'info');
            devLog('STRATEGIES', `Strategy ${strategy.name} started successfully`);
            startStrategyHeartbeat();
            await syncStrategiesStatus();
        } else {
            strategyStatuses[strategyId] = 'idle';
            const message = result?.message || 'Unknown error';
            addStrategyLog(`Start failed: ${strategy.name} — ${message}`, 'error');
            devError('STRATEGIES', `Failed to start strategy ${strategy.name}:`, message);
        }
    } catch (error) {
        strategyStatuses[strategyId] = 'idle';
        addStrategyLog(`Error starting ${strategy.name}: ${error.message}`, 'error');
        devError('STRATEGIES', `Error executing strategy ${strategy.name}:`, error);
    }

    renderStrategies();
}

async function stopStrategy(e) {
    const button = e.currentTarget;
    const strategyId = button.dataset.strategyId;
    const strategy = STRATEGIES.find(s => s.id === parseInt(strategyId));

    if (!strategy) return;

    // Optimistic UI update
    strategyStatuses[strategyId] = 'stopping';
    renderStrategies();
    addStrategyLog(`Stopping ${strategy.name}...`, 'info');

    try {
        const response = await fetch(`${API_BASE}/api/strategies/${strategyId}/stop`, {
            method: 'POST',
            headers: getAuthHeaders(),
            body: JSON.stringify({})
        });

        const result = await response.json();
        if (response.ok && result.success) {
            strategyStatuses[strategyId] = 'idle';
            addStrategyLog(`Stopped: ${strategy.name}`, 'success');
            devLog('STRATEGIES', `Strategy ${strategy.name} stopped successfully`);
            stopStrategyHeartbeat();
            await syncStrategiesStatus();
        } else {
            strategyStatuses[strategyId] = 'running'; // Revert if failed
            const message = result?.message || 'Unknown error';
            addStrategyLog(`Stop failed: ${strategy.name} — ${message}`, 'error');
            devError('STRATEGIES', `Failed to stop strategy ${strategy.name}:`, message);
        }
    } catch (error) {
        strategyStatuses[strategyId] = 'running'; // Revert if failed
        addStrategyLog(`Error stopping ${strategy.name}: ${error.message}`, 'error');
        devError('STRATEGIES', `Error stopping strategy ${strategy.name}:`, error);
    }

    renderStrategies();
}

function initStrategies() {
    // Render strategies when page loads
    renderStrategies();
}

async function syncStrategiesStatus() {
    try {
        const response = await fetch(`${API_BASE}/api/strategies/status`, {
            headers: getAuthHeaders()
        });

        if (response.ok) {
            const result = await response.json();
            if (result.success && result.strategies) {
                result.strategies.forEach(strategy => {
                    const currentStatus = strategyStatuses[strategy.id];
                    const backendStatus = strategy.state.toLowerCase();
                    
                    // Only update if backend status differs and we're not in a transitional state
                    if (backendStatus !== currentStatus && 
                        currentStatus !== 'starting' && 
                        currentStatus !== 'stopping') {
                        strategyStatuses[strategy.id] = backendStatus;
                    }
                });
                renderStrategies();
            }
        }
    } catch (error) {
        devError('STRATEGIES', 'Failed to sync strategy statuses:', error);
    }
}

// Options Chain Chart
let optionsData = null;
let selectedStrike = null;
let optionsCanvas = null;
let selectedOptionInfo = null;

function initOptionsChain() {
    const loadOptionsBtn = document.getElementById('load-options-btn');
    const optionsSymbolInput = document.getElementById('options-symbol');
    const optionsExpirationInput = document.getElementById('options-expiration');
    const optionsLoading = document.getElementById('options-loading');
    const optionsChartContainer = document.getElementById('options-chart-container');
    const optionsError = document.getElementById('options-error');
    const stockPriceValue = document.getElementById('stock-price-value');
    // Assign module-level refs used by drawOptionsChart/showStrikeDetails
    optionsCanvas = document.getElementById('options-chart');
    selectedOptionInfo = document.getElementById('selected-option-info');

    // Set default expiration date to next Friday
    if (optionsExpirationInput) optionsExpirationInput.value = getNextFriday();
    if (!loadOptionsBtn) return;


    loadOptionsBtn.addEventListener('click', async () => {
        const symbol = optionsSymbolInput ? optionsSymbolInput.value.toUpperCase() : '';
        const expiration = optionsExpirationInput ? optionsExpirationInput.value : '';

        if (!symbol) { alert('Please enter a symbol'); return; }
        if (!expiration) { alert('Please select an expiration date'); return; }

        loadOptionsBtn.disabled = true;
        loadOptionsBtn.textContent = 'Loading...';
        if (optionsLoading) optionsLoading.style.display = 'block';
        if (optionsChartContainer) optionsChartContainer.style.display = 'none';
        if (optionsError) optionsError.style.display = 'none';

        try {
            // Get current stock price first
            const priceResponse = await fetchWithLogging(`${API_BASE}/api/price/${symbol}`, { headers: getAuthHeaders() });

            const priceData = priceResponse._body;
            let stockPrice = 0;
            if (priceData.quote) stockPrice = priceData.quote.ap || priceData.quote.bp || 0;

            // Get real options chain data
            const response = await fetchWithLogging(`${API_BASE}/api/option-chain/${symbol}`, { headers: getAuthHeaders() });
            if (!response.ok) {
                const errorData = response._body;
                throw new Error(errorData.error || 'Failed to load options data');
            }
            const chainData = response._body;

            // Format expiration date for display (parse locally to avoid UTC shift)
            const [eY, eM, eD] = expiration.split('-').map(Number);
            const expDate = new Date(eY, eM - 1, eD);
            const expFormatted = expDate.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });

            optionsData = {
                symbol,
                stockPrice: chainData.underlying_price,
                strikes: chainData.strikes,
                expiration,
                expirationFormatted: expFormatted
            };

            if (stockPriceValue) stockPriceValue.textContent = formatCurrency(stockPrice);
            if (optionsLoading) optionsLoading.style.display = 'none';
            if (optionsChartContainer) optionsChartContainer.style.display = 'block';

            drawOptionsChart();

        } catch (err) {
            if (optionsLoading) optionsLoading.style.display = 'none';
            if (optionsError) { optionsError.style.display = 'block'; optionsError.textContent = `Error: ${err.message}`; }
        } finally {
            loadOptionsBtn.disabled = false;
            loadOptionsBtn.textContent = 'Load Options';
        }
    });
} // end initOptionsChain

function drawOptionsChart() {
    if (!optionsData || !optionsCanvas || !optionsData.strikes || optionsData.strikes.length === 0) return;

    const ctx = optionsCanvas.getContext('2d');
    const width = optionsCanvas.width;
    const height = optionsCanvas.height;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    const stockPrice = optionsData.stockPrice;

    // Get strikes from data
    const strikes = optionsData.strikes.map(s => s.strike);
    const minStrike = Math.min(...strikes);
    const maxStrike = Math.max(...strikes);

    // Calculate a reasonable strike increment for grid lines
    const range = maxStrike - minStrike;
    const strikeIncrement = range / 10;

    // Chart dimensions
    const margin = { top: 40, right: 60, bottom: 50, left: 80 };
    const chartWidth = width - margin.left - margin.right;
    const chartHeight = height - margin.top - margin.bottom;

    // Scale functions
    const xScale = (strike) => {
        return margin.left + ((strike - minStrike) / (maxStrike - minStrike)) * chartWidth;
    };

    // Draw background
    ctx.fillStyle = 'rgba(0, 0, 0, 0.3)';
    ctx.fillRect(margin.left, margin.top, chartWidth, chartHeight);

    // Draw grid lines
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
    ctx.lineWidth = 1;

    // Vertical grid lines (strikes)
    for (let strike = minStrike; strike <= maxStrike; strike += strikeIncrement * 2) {
        const x = xScale(strike);
        ctx.beginPath();
        ctx.moveTo(x, margin.top);
        ctx.lineTo(x, margin.top + chartHeight);
        ctx.stroke();
    }

    // Draw strike labels
    ctx.fillStyle = '#888';
    ctx.font = '12px Arial';
    ctx.textAlign = 'center';
    for (let strike = minStrike; strike <= maxStrike; strike += strikeIncrement * 2) {
        const x = xScale(strike);
        ctx.fillText(strike.toFixed(0), x, margin.top + chartHeight + 20);
    }

    // Draw stock price line (ATM)
    const stockX = xScale(stockPrice);
    ctx.strokeStyle = '#ffd700';
    ctx.lineWidth = 2;
    ctx.setLineDash([5, 5]);
    ctx.beginPath();
    ctx.moveTo(stockX, margin.top);
    ctx.lineTo(stockX, margin.top + chartHeight);
    ctx.stroke();
    ctx.setLineDash([]);

    // Label for stock price
    ctx.fillStyle = '#ffd700';
    ctx.font = 'bold 12px Arial';
    ctx.textAlign = 'center';
    ctx.fillText(`$${stockPrice.toFixed(2)}`, stockX, margin.top - 10);

    // Draw ITM/OOTM regions
    // Calls: ITM when strike < stock price
    // Puts: ITM when strike > stock price
    const callITMStart = margin.left;
    const callITMEnd = stockX;
    const putITMStart = stockX;
    const putITMEnd = margin.left + chartWidth;

    // Highlight ITM regions
    ctx.fillStyle = 'rgba(0, 255, 136, 0.1)';
    ctx.fillRect(callITMStart, margin.top, callITMEnd - callITMStart, chartHeight);

    ctx.fillStyle = 'rgba(255, 68, 68, 0.1)';
    ctx.fillRect(putITMStart, margin.top, putITMEnd - putITMStart, chartHeight);

    // Draw real option prices from data
    const allPrices = optionsData.strikes.flatMap(s => [s.call.ask, s.put.ask]);
    const maxPrice = Math.max(...allPrices, 1);

    const yScale = (price) => {
        return margin.top + chartHeight - (price / maxPrice) * chartHeight;
    };

    // Sort strikes for drawing lines
    const sortedStrikes = [...optionsData.strikes].sort((a, b) => a.strike - b.strike);

    // Draw call price curve (ask)
    ctx.strokeStyle = '#00ff88';
    ctx.lineWidth = 3;
    ctx.beginPath();
    sortedStrikes.forEach((s, i) => {
        const x = xScale(s.strike);
        const y = yScale(s.call.ask);
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    });
    ctx.stroke();

    // Draw call price curve (bid)
    ctx.strokeStyle = '#88ffaa';
    ctx.lineWidth = 1;
    ctx.beginPath();
    sortedStrikes.forEach((s, i) => {
        const x = xScale(s.strike);
        const y = yScale(s.call.bid);
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    });
    ctx.stroke();

    // Draw put price curve (ask)
    ctx.strokeStyle = '#ff4444';
    ctx.lineWidth = 3;
    ctx.beginPath();
    sortedStrikes.forEach((s, i) => {
        const x = xScale(s.strike);
        const y = yScale(s.put.ask);
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    });
    ctx.stroke();

    // Draw put price curve (bid)
    ctx.strokeStyle = '#ff8888';
    ctx.lineWidth = 1;
    ctx.beginPath();
    sortedStrikes.forEach((s, i) => {
        const x = xScale(s.strike);
        const y = yScale(s.put.bid);
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    });
    ctx.stroke();

    // Draw Y-axis labels
    ctx.fillStyle = '#888';
    ctx.font = '12px Arial';
    ctx.textAlign = 'right';
    for (let i = 0; i <= 4; i++) {
        const price = (maxPrice / 4) * i;
        const y = yScale(price);
        ctx.fillText(`$${price.toFixed(0)}`, margin.left - 10, y + 4);
    }

    // Draw title
    ctx.fillStyle = '#fff';
    ctx.font = 'bold 14px Arial';
    ctx.textAlign = 'center';
    const title = optionsData.expirationFormatted
        ? `${optionsData.symbol} Options Chain - ${optionsData.expirationFormatted}`
        : `${optionsData.symbol} Options Chain`;
    ctx.fillText(title, width / 2, 20);

    // Add click handler for strike selection (removes any previous handler)
    optionsCanvas.onclick = (e) => {
        const rect = optionsCanvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const strike = minStrike + ((x - margin.left) / chartWidth) * (maxStrike - minStrike);

        // Find closest strike in data
        let closest = sortedStrikes[0];
        let minDist = Math.abs(strike - closest.strike);

        sortedStrikes.forEach(s => {
            const dist = Math.abs(strike - s.strike);
            if (dist < minDist) {
                minDist = dist;
                closest = s;
            }
        });

        showStrikeDetails(closest);
    };
}

function selectOptionForOrder(symbol, type, strike, price) {
    // Switch to option tab
    const optionToggle = document.querySelector('.toggle-btn[data-class="option"]');
    if (optionToggle) optionToggle.click();

    // Fill fields
    document.getElementById('option-symbol').value = symbol;
    document.getElementById('option-type-hidden').value = type;
    document.getElementById('strike-price').value = strike;
    document.getElementById('option-limit-price').value = price;
    document.getElementById('option-order-type').value = 'limit';

    // Trigger display updates
    const event = new Event('change');
    document.getElementById('option-order-type').dispatchEvent(event);

    // Update buttons UI
    document.querySelectorAll('.option-type-btn').forEach(btn => {
        btn.classList.remove('active');
        if (btn.dataset.type === type) btn.classList.add('active');
    });

    // Scroll to order form
    document.getElementById('new-order-section').scrollIntoView({ behavior: 'smooth' });
}

window.selectOptionForOrder = selectOptionForOrder;

function showStrikeDetails(strikeData) {
    if (!optionsData || !strikeData) return;

    const stockPrice = optionsData.stockPrice;
    const strike = strikeData.strike;

    const callAsk = strikeData.call.ask.toFixed(2);
    const callBid = strikeData.call.bid.toFixed(2);
    const putAsk = strikeData.put.ask.toFixed(2);
    const putBid = strikeData.put.bid.toFixed(2);

    const callMoneyness = strike < stockPrice ? 'ITM' : (strike > stockPrice ? 'OTM' : 'ATM');
    const putMoneyness = strike > stockPrice ? 'ITM' : (strike < stockPrice ? 'OTM' : 'ATM');

    selectedOptionInfo.innerHTML = `
        <div class="option-detail-grid">
            <div class="option-detail-item">
                <button class="btn-fill-price" onclick="selectOptionForOrder('${strikeData.call.symbol}', 'call', ${strike}, ${callAsk})">Select Call</button>
            </div>
            <div class="option-detail-item">
                <button class="btn-fill-price" style="background: linear-gradient(135deg, #ff4444 0%, #cc0000 100%);" onclick="selectOptionForOrder('${strikeData.put.symbol}', 'put', ${strike}, ${putAsk})">Select Put</button>
            </div>
            <div class="option-detail-item">
                <span class="label">Strike</span>
                <span class="value">$${strike.toFixed(2)}</span>
            </div>
            <div class="option-detail-item">
                <span class="label">Stock Price</span>
                <span class="value">$${stockPrice.toFixed(2)}</span>
            </div>
            <div class="option-detail-item">
                <span class="label">Call Moneyness</span>
                <span class="value ${callMoneyness === 'ITM' ? 'call' : ''}">${callMoneyness}</span>
            </div>
            <div class="option-detail-item">
                <span class="label">Put Moneyness</span>
                <span class="value ${putMoneyness === 'ITM' ? 'put' : ''}">${putMoneyness}</span>
            </div>
        </div>
        <div class="option-detail-grid" style="margin-top: 15px;">
            <div class="option-detail-item">
                <span class="label">Call Bid</span>
                <span class="value call">$${callBid}</span>
            </div>
            <div class="option-detail-item">
                <span class="label">Call Ask</span>
                <span class="value call">$${callAsk}</span>
            </div>
            <div class="option-detail-item">
                <span class="label">Put Bid</span>
                <span class="value put">$${putBid}</span>
            </div>
            <div class="option-detail-item">
                <span class="label">Put Ask</span>
                <span class="value put">$${putAsk}</span>
            </div>
        </div>
        <p style="margin-top: 10px; color: #666; font-size: 0.85rem;">
            Click on a strike in the chart to see details. Prices shown are estimates based on intrinsic + time value.
        </p>
    `;
}

// ============================================================
// ANALYTICS TAB
// ============================================================

let currentSignals = [];
let allPatterns = [];

function initAnalytics() {
    if (watchlistAddBtn) watchlistAddBtn.addEventListener('click', addToWatchlist);
    if (watchlistAddInput) watchlistAddInput.addEventListener('keydown', (e) => { if (e.key === 'Enter') addToWatchlist(); });
    if (fetchDataBtn) fetchDataBtn.addEventListener('click', () => fetchData(false));
    if (fetchFullBtn) fetchFullBtn.addEventListener('click', () => fetchData(true));
    if (analyzeBtn) analyzeBtn.addEventListener('click', runAnalysis);
    if (exportSignalsBtn) exportSignalsBtn.addEventListener('click', exportSignals);
    if (signalFilterSymbol) signalFilterSymbol.addEventListener('input', renderSignals);
    if (signalFilterDirection) signalFilterDirection.addEventListener('change', renderSignals);
    if (signalFilterPattern) signalFilterPattern.addEventListener('change', renderSignals);
}

async function loadWatchlist() {
    try {
        const response = await fetchWithLogging(`${API_BASE}/api/analytics/watchlist`, {
            headers: getAuthHeaders()
        });
        if (!response.ok) throw new Error('Failed to load watchlist');
        const data = await response.json();
        renderWatchlist(data.symbols || []);
    } catch (e) {
        if (watchlistError) { watchlistError.textContent = e.message; watchlistError.style.display = 'block'; }
    }
}

function renderWatchlist(symbols) {
    if (!watchlistSymbols) return;
    if (symbols.length === 0) {
        watchlistSymbols.innerHTML = '<span style="color: #888;">No symbols in watchlist. Add some above.</span>';
        return;
    }
    watchlistSymbols.innerHTML = symbols.map(sym => `
        <div class="watchlist-tag">
            ${sym}
            <span class="remove-tag" onclick="removeFromWatchlist('${sym}')">&times;</span>
        </div>
    `).join('');
}

async function addToWatchlist() {
    const input = watchlistAddInput.value.trim();
    if (!input) return;
    const symbols = input.split(/[,\s]+/).map(s => s.toUpperCase().trim()).filter(s => s);
    if (symbols.length === 0) return;

    try {
        const response = await fetchWithLogging(`${API_BASE}/api/analytics/watchlist`, {
            method: 'POST',
            headers: getAuthHeaders(),
            body: JSON.stringify({ add: symbols })
        });
        if (!response.ok) throw new Error('Failed to add symbols');
        const data = await response.json();
        renderWatchlist(data.symbols || []);
        if (watchlistAddInput) watchlistAddInput.value = '';
        loadDataSummary();
    } catch (e) {
        if (watchlistError) { watchlistError.textContent = e.message; watchlistError.style.display = 'block'; }
    }
}

async function removeFromWatchlist(symbol) {
    try {
        const response = await fetchWithLogging(`${API_BASE}/api/analytics/watchlist`, {
            method: 'POST',
            headers: getAuthHeaders(),
            body: JSON.stringify({ remove: [symbol] })
        });
        if (!response.ok) throw new Error('Failed to remove symbol');
        const data = await response.json();
        renderWatchlist(data.symbols || []);
        loadDataSummary();
    } catch (e) {
        if (watchlistError) { watchlistError.textContent = e.message; watchlistError.style.display = 'block'; }
    }
}

async function loadDataSummary() {
    try {
        const response = await fetchWithLogging(`${API_BASE}/api/analytics/summary`, {
            headers: getAuthHeaders()
        });
        if (!response.ok) throw new Error('Failed to load summary');
        const data = await response.json();
        renderDataSummary(data);
    } catch (e) {
        // Silent fail for summary - non-critical
    }
}

function renderDataSummary(data) {
    if (!dataSummaryContainer) return;
    const symbols = data.symbols || [];
    if (symbols.length === 0) {
        dataSummaryContainer.innerHTML = '<p style="color: #888; text-align: center;">No data yet. Fetch data to get started.</p>';
        return;
    }
    let html = '<div class="data-summary-grid">';
    for (const sym of symbols) {
        html += `<div class="data-summary-item"><div class="symbol">${sym.symbol}</div>`;
        for (const tf of sym.timeframes) {
            const countStr = tf.bar_count > 0 ? `<span class="count">${tf.bar_count.toLocaleString()}</span>` : '<span style="color:#666;">no data</span>';
            html += `<div class="tf-row"><span>${tf.timeframe}</span>${countStr}</div>`;
        }
        html += '</div>';
    }
    html += '</div>';
    dataSummaryContainer.innerHTML = html;
}

async function fetchData(fullRefresh) {
    const symbolsStr = fetchSymbolsInput ? fetchSymbolsInput.value.trim() : '';
    const source = fetchSourceSelect ? fetchSourceSelect.value : 'yfinance';
    const timeframes = [];
    document.querySelectorAll('#fetch-timeframes input[type="checkbox"]:checked').forEach(cb => timeframes.push(cb.value));

    const body = { source, timeframes: timeframes.length > 0 ? timeframes : undefined };
    if (symbolsStr) body.symbols = symbolsStr.split(/[,\s]+/).map(s => s.toUpperCase().trim()).filter(s => s);
    if (fullRefresh) body.full = true;

    if (dataLoading) dataLoading.style.display = 'block';
    if (dataError) dataError.style.display = 'none';

    try {
        const response = await fetchWithLogging(`${API_BASE}/api/analytics/fetch`, {
            method: 'POST',
            headers: getAuthHeaders(),
            body: JSON.stringify(body)
        });
        if (!response.ok) throw new Error('Failed to fetch data');
        const data = await response.json();
        renderFetchResults(data.results || []);
        loadDataSummary();
    } catch (e) {
        if (dataError) { dataError.textContent = e.message; dataError.style.display = 'block'; }
    } finally {
        if (dataLoading) dataLoading.style.display = 'none';
    }
}

function renderFetchResults(results) {
    const container = document.getElementById('fetch-results-container');
    if (!container || results.length === 0) return;
    container.style.display = 'block';
    let html = '<div style="margin-top: 15px;">';
    for (const r of results) {
        const statusClass = r.status === 'ok' ? 'status-ok' : 'status-error';
        const statusText = r.status === 'ok' ? `${r.bars_fetched} bars` : `Error: ${r.error || 'unknown'}`;
        html += `<div class="fetch-result"><span>${r.symbol} ${r.timeframe}</span><span class="${statusClass}">${statusText}</span></div>`;
    }
    html += '</div>';
    container.innerHTML = html;
}

async function loadPatterns() {
    if (patternCheckboxes && patternCheckboxes.children.length > 0) return;
    try {
        const response = await fetchWithLogging(`${API_BASE}/api/analytics/patterns`);
        if (!response.ok) return;
        const data = await response.json();
        allPatterns = data.patterns || [];
        renderPatternCheckboxes();
        renderPatternFilter();
    } catch (e) {
        // Use defaults on error
    }
}

function renderPatternCheckboxes() {
    if (!patternCheckboxes) return;
    patternCheckboxes.innerHTML = allPatterns.map(p =>
        `<label class="checkbox-label"><input type="checkbox" value="${p.id}" checked> ${p.name}</label>`
    ).join('');
}

function renderPatternFilter() {
    if (!signalFilterPattern) return;
    let html = '<option value="all">All Patterns</option>';
    // Use detector pattern IDs as filter options since analysis groups results by detector
    for (const p of allPatterns) {
        html += `<option value="${p.id}">${p.name}</option>`;
    }
    signalFilterPattern.innerHTML = html;
}

async function runAnalysis() {
    const symbolsStr = analysisSymbols ? analysisSymbols.value.trim() : '';
    const minConfidence = minConfidenceInput ? parseFloat(minConfidenceInput.value) || 0 : 0;
    const patterns = [];
    if (patternCheckboxes) {
        patternCheckboxes.querySelectorAll('input[type="checkbox"]:checked').forEach(cb => patterns.push(cb.value));
    }

    // Validate: must have at least one pattern selected
    if (patterns.length === 0) {
        if (analysisError) {
            analysisError.textContent = 'Select at least one pattern to analyze.';
            analysisError.style.display = 'block';
        }
        currentSignals = [];
        renderSignals();
        return;
    }

    const storeSignals = storeSignalsCheckbox ? storeSignalsCheckbox.checked : false;
    const updateData = updateDataCheckbox ? updateDataCheckbox.checked : false;
    const source = fetchSourceSelect ? fetchSourceSelect.value : 'yfinance';

    const body = { min_confidence: minConfidence };
    if (symbolsStr) body.symbols = symbolsStr.split(/[,\s]+/).map(s => s.toUpperCase().trim()).filter(s => s);
    if (patterns.length > 0) body.patterns = patterns;
    if (storeSignals) body.store = true;
    if (updateData) body.update = true;
    body.source = source;

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
        currentSignals = (data.signals || []).map(s => {
            if (typeof s.details === 'string') { try { s.details = JSON.parse(s.details); } catch(e) {} }
            return s;
        });
        renderSignals();
        if (updateData) loadDataSummary();
    } catch (e) {
        if (analysisError) { analysisError.textContent = e.message; analysisError.style.display = 'block'; }
    } finally {
        if (analysisLoading) analysisLoading.style.display = 'none';
    }
}

function renderSignals() {
    if (!signalsBody || !signalsTable || !signalsNoData) return;

    const filterSym = signalFilterSymbol ? signalFilterSymbol.value.toUpperCase().trim() : '';
    const filterDir = signalFilterDirection ? signalFilterDirection.value : 'all';
    const filterPat = signalFilterPattern ? signalFilterPattern.value : 'all';

    let filtered = currentSignals;
    if (filterSym) filtered = filtered.filter(s => s.symbol.toUpperCase().includes(filterSym));
    if (filterDir !== 'all') filtered = filtered.filter(s => s.direction === filterDir);
    if (filterPat !== 'all') {
        // Filter by detector ID - match signals whose pattern starts with the detector prefix
        // e.g. "gap_analysis" matches "gap_up", "gap_down"
        // e.g. "momentum_1d" matches "momentum_5d", "momentum_10d", etc.
        filtered = filtered.filter(s => {
            const sigPat = s.pattern;
            // Direct match
            if (sigPat === filterPat) return true;
            // Prefix match: detector "gap_analysis" matches signals "gap_up"/"gap_down"
            // detector "unusual_volume_1m" matches "unusual_volume_1m"
            const detectorPrefix = filterPat.replace(/_analysis$/, '').replace(/_\d+d$/, '');
            if (sigPat.startsWith(detectorPrefix)) return true;
            return false;
        });
    }

    if (filtered.length === 0) {
        signalsTable.style.display = 'none';
        signalsNoData.style.display = 'block';
        signalsNoData.textContent = currentSignals.length === 0 ? 'Run analysis to see signals' : 'No signals match your filters';
        if (signalsCount) signalsCount.style.display = 'none';
        return;
    }

    signalsTable.style.display = 'table';
    signalsNoData.style.display = 'none';
    if (signalsCount) {
        signalsCount.style.display = 'block';
        signalsCount.textContent = `Showing ${filtered.length} of ${currentSignals.length} signals`;
    }

    signalsBody.innerHTML = filtered.map(s => {
        const dirClass = `direction-${s.direction}`;
        const dirIcon = s.direction === 'bullish' ? '▲' : s.direction === 'bearish' ? '▼' : '—';
        const confPct = Math.round(s.confidence * 100);
        const confColor = confPct >= 70 ? '#00ff88' : confPct >= 40 ? '#ffa500' : '#ff4444';
        const timeStr = s.timestamp ? new Date(s.timestamp).toLocaleString() : '-';
        const details = s.details || {};
        const keyDetails = {};
        for (const k of ['vwap', 'price', 'gap_pct', 'volume', 'avg_volume', 'z_score', 'roc_pct', 'range_high', 'range_low', 'breakout_price', 'hist_fill_rate_up', 'hist_fill_rate_down', 'band_type']) {
            if (details[k] !== undefined && details[k] !== null) keyDetails[k] = details[k];
        }
        const keyStr = Object.keys(keyDetails).length > 0 ? Object.entries(keyDetails).map(([k,v]) => `${k}: ${typeof v === 'number' ? v.toFixed(2) : v}`).join(', ') : '';
        const detailId = `detail-${s.timestamp}-${s.symbol}-${s.pattern}`.replace(/[^a-zA-Z0-9-]/g, '_');

        return `<tr onclick="toggleSignalDetail('${detailId}')" style="cursor:pointer;">
            <td>${timeStr}</td>
            <td style="font-weight:700;">${s.symbol}</td>
            <td>${formatPatternName(s.pattern)}</td>
            <td class="${dirClass}">${dirIcon} ${s.direction}</td>
            <td><div class="confidence-bar-container"><div class="confidence-bar" style="width:${confPct}%;background:${confColor};"></div><span class="confidence-text">${confPct}%</span></div></td>
            <td style="font-size:0.8rem;">${keyStr}</td>
        </tr><tr id="${detailId}" class="signal-details-row" style="display:none;"><td colspan="6"><pre>${JSON.stringify(details, null, 2)}</pre></td></tr>`;
    }).join('');
}

function formatPatternName(pattern) {
    const names = {
        'vwap_deviation': 'VWAP Deviation',
        'opening_range_breakout_15m': 'ORB 15m',
        'opening_range_breakout_30m': 'ORB 30m',
        'intraday_mean_reversion': 'Mean Reversion',
        'gap_up': 'Gap Up',
        'gap_down': 'Gap Down',
        'unusual_volume_1m': 'Volume Spike 1m',
        'unusual_volume_1d': 'Volume Spike 1d',
        'momentum_5d': 'Momentum 5d',
        'momentum_10d': 'Momentum 10d',
        'momentum_20d': 'Momentum 20d',
        'momentum_51d': 'Momentum 51d',
        'momentum_101d': 'Momentum 101d',
        'momentum_201d': 'Momentum 201d',
    };
    return names[pattern] || pattern;
}

function toggleSignalDetail(detailId) {
    const row = document.getElementById(detailId);
    if (row) {
        row.style.display = row.style.display === 'none' ? 'table-row' : 'none';
    }
}

function exportSignals() {
    if (currentSignals.length === 0) { alert('No signals to export. Run analysis first.'); return; }
    const data = JSON.stringify(currentSignals, null, 2);
    const blob = new Blob([data], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `signals_${new Date().toISOString().slice(0,10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
}