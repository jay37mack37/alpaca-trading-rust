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
    fetch(`${API_BASE}/api/verify`, {
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
            localStorage.removeItem('token');
            localStorage.removeItem('username');
            window.location.href = '/login.html';
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

// History Elements
let historyBody, noHistory, historyTable;
let filterSymbol, filterSide, filterStartDate, filterEndDate;

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

    historyBody = document.getElementById('history-body');
    noHistory = document.getElementById('no-history');
    historyTable = document.getElementById('history-table');
    filterSymbol = document.getElementById('filter-symbol');
    filterSide = document.getElementById('filter-side');
    filterStartDate = document.getElementById('filter-start-date');
    filterEndDate = document.getElementById('filter-end-date');
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
        const response = await fetch(`${API_BASE}/api/account`, {
            headers: getAuthHeaders()
        });

        if (response.status === 401) {
            localStorage.removeItem('token');
            window.location.href = '/login.html';
            return;
        }

        const data = await response.json();

        if (!response.ok) {
            throw new Error(data.message || 'Failed to fetch account');
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
        accountError.textContent = `Error: ${err.message}. Configure API keys in Settings.`;
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
        const response = await fetch(`${API_BASE}/api/positions`, {
            headers: getAuthHeaders()
        });

        if (response.status === 401) {
            return; // Auth check handles redirect
        }

        const data = await response.json();

        if (!response.ok) {
            throw new Error(data.message || 'Failed to fetch positions');
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

    const cancelAllBtn = document.getElementById('cancel-all-orders-btn');
    const cancelSelectedBtn = document.getElementById('cancel-selected-btn');
    const selectAllCheckbox = document.getElementById('select-all-orders');

    try {
        const response = await fetch(`${API_BASE}/api/orders`, {
            headers: getAuthHeaders()
        });

        if (response.status === 401) {
            return;
        }

        const data = await response.json();

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
                    ${isOpen ? `<button class="btn-cancel" onclick="cancelOrder('${order.id}')">Cancel</button>` : ''}
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
    const cancelSelectedBtn = document.getElementById('cancel-selected-btn');
    const selectAllCheckbox = document.getElementById('select-all-orders');
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
    const selectAllCheckbox = document.getElementById('select-all-orders');
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
        const response = await fetch(`${API_BASE}/api/orders/${orderId}`, {
            method: 'DELETE',
            headers: getAuthHeaders()
        });

        const data = await response.json();

        if (!response.ok) {
            throw new Error(data.error || data.message || 'Failed to cancel order');
        }

        // Log the cancellation
        logTransaction(data, 'cancelled');

        alert('Order cancelled successfully!');
        fetchOrders();
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
            const response = await fetch(`${API_BASE}/api/orders/${orderId}`, {
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
        const response = await fetch(`${API_BASE}/api/orders/cancel-all`, {
            method: 'POST',
            headers: getAuthHeaders()
        });

        const data = await response.json();

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
window.cancelSelectedOrders = cancelSelectedOrders;
window.toggleSelectAllOrders = toggleSelectAllOrders;
window.updateCancelSelectedButton = updateCancelSelectedButton;

function initOrderButtons() {
    const cancelAllBtn = document.getElementById('cancel-all-orders-btn');
    if (cancelAllBtn) cancelAllBtn.addEventListener('click', cancelAllOrders);

    const cancelSelectedBtn = document.getElementById('cancel-selected-btn');
    if (cancelSelectedBtn) cancelSelectedBtn.addEventListener('click', cancelSelectedOrders);

    const selectAllCheckbox = document.getElementById('select-all-orders');
    if (selectAllCheckbox) selectAllCheckbox.addEventListener('change', toggleSelectAllOrders);
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
                    const qty = parseFloat(document.getElementById('qty').value);
                    const orderType = document.getElementById('order-type').value;
                    const limitPrice = document.getElementById('limit-price').value;
                    const timeInForce = document.getElementById('time-in-force').value;

                    orderData = { symbol, qty, side, order_type: orderType, time_in_force: timeInForce };
                    if (orderType === 'limit' && limitPrice) orderData.limit_price = parseFloat(limitPrice);
                } else {
                    // Option order
                    const underlying = document.getElementById('option-symbol').value.toUpperCase();
                    const optionType = document.getElementById('option-type-hidden').value;
                    const strike = parseFloat(document.getElementById('strike-price').value);
                    const expiration = document.getElementById('expiration-date').value;
                    const side = document.getElementById('option-side').value;
                    const qty = parseInt(document.getElementById('option-qty').value);
                    const orderType = document.getElementById('option-order-type').value;
                    const limitPrice = document.getElementById('option-limit-price').value;
                    const timeInForce = document.getElementById('option-tif').value;

                    if (!optionType) throw new Error('Please select Call or Put');

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
                    if (orderType === 'limit' && limitPrice) orderData.limit_price = parseFloat(limitPrice);
                }

                const response = await fetch(`${API_BASE}/api/orders`, {
                    method: 'POST',
                    headers: getAuthHeaders(),
                    body: JSON.stringify(orderData)
                });

                if (response.status === 401) {
                    localStorage.removeItem('token');
                    window.location.href = '/login.html';
                    return;
                }

                const data = await response.json();
                if (!response.ok) throw new Error(data.message || data.error || 'Failed to place order');

                // Log the placed order
                logTransaction(data, 'placed');

                orderSuccess.style.display = 'block';
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
                const response = await fetch(`${API_BASE}/api/price/${symbol}`, { headers: getAuthHeaders() });
                if (response.status === 401) { localStorage.removeItem('token'); window.location.href = '/login.html'; return; }
                const data = await response.json();
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

                const response = await fetch(`${API_BASE}/api/option-strikes/${optionSymbol.replace(/\s/g, '')}`, { headers: getAuthHeaders() });
                if (response.status === 401) { localStorage.removeItem('token'); window.location.href = '/login.html'; return; }
                const data = await response.json();
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
    const response = await fetch(`${API_BASE}/api/option-quote/${underlying}`, { headers: getAuthHeaders() });
    if (response.status === 401) { localStorage.removeItem('token'); window.location.href = '/login.html'; return null; }
    const data = await response.json();
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
        status: order.status,
        event: eventType,
        timestamp: new Date().toISOString()
    };

    // Check if this specific event for this order is already logged to avoid duplicates
    const isDuplicate = history.some(h => h.orderId === order.id && h.event === eventType && h.status === order.status);
    if (isDuplicate) return;

    history.unshift(entry);
    saveHistory(history);
    devLog('HISTORY', `Logged ${eventType} for ${order.symbol}`, entry);
}

// Backfill history from API
async function backfillHistory() {
    devLog('HISTORY', 'Backfilling history from API...');
    try {
        const response = await fetch(`${API_BASE}/api/orders?status=all`, {
            headers: getAuthHeaders()
        });

        if (!response.ok) throw new Error('Failed to fetch historical orders');
        const orders = await response.json();

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
        const response = await fetch(`${API_BASE}/api/orders`, {
            headers: getAuthHeaders()
        });
        if (!response.ok) return;
        const currentOrders = await response.json();

        const history = getHistory();

        currentOrders.forEach(order => {
            // Check if we have a log for this order's current status
            const hasStatusLog = history.some(h => h.orderId === order.id && h.status === order.status);
            if (!hasStatusLog) {
                let eventType = 'status_change';
                if (order.status === 'filled') eventType = 'filled';
                if (order.status === 'canceled') eventType = 'cancelled';
                if (order.status === 'expired') eventType = 'expired';

                logTransaction(order, eventType);
            }
        });
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
        });
    });
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
            const priceResponse = await fetch(`${API_BASE}/api/price/${symbol}`, { headers: getAuthHeaders() });
            if (priceResponse.status === 401) { localStorage.removeItem('token'); window.location.href = '/login.html'; return; }

            const priceData = await priceResponse.json();
            let stockPrice = 0;
            if (priceData.quote) stockPrice = priceData.quote.ap || priceData.quote.bp || 0;

            // Get real options chain data
            const response = await fetch(`${API_BASE}/api/option-chain/${symbol}`, { headers: getAuthHeaders() });
            if (!response.ok) {
                const errorData = await response.json();
                throw new Error(errorData.error || 'Failed to load options data');
            }
            const chainData = await response.json();

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

    // Add click handler for strike selection
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