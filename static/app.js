// API Base URL
const API_BASE = window.location.origin;

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

// DOM Elements
const statusDot = document.querySelector('.status-dot');
const statusText = document.getElementById('status-text');

// Account Elements
const accountLoading = document.getElementById('account-loading');
const accountInfo = document.getElementById('account-info');
const accountError = document.getElementById('account-error');

// Positions Elements
const positionsLoading = document.getElementById('positions-loading');
const positionsTable = document.getElementById('positions-table');
const positionsBody = document.getElementById('positions-body');
const noPositions = document.getElementById('no-positions');
const positionsError = document.getElementById('positions-error');

// Orders Elements
const ordersLoading = document.getElementById('orders-loading');
const ordersTable = document.getElementById('orders-table');
const ordersBody = document.getElementById('orders-body');
const noOrders = document.getElementById('no-orders');
const ordersError = document.getElementById('orders-error');

// Order Form Elements
const orderForm = document.getElementById('order-form');
const orderSuccess = document.getElementById('order-success');
const orderError = document.getElementById('order-error');
const submitBtn = document.getElementById('submit-order');

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

    try {
        const response = await fetch(`${API_BASE}/api/orders`, {
            headers: getAuthHeaders()
        });

        if (response.status === 401) {
            return;
        }

        const data = await response.json();

        if (!response.ok) {
            throw new Error(data.message || 'Failed to fetch orders');
        }

        ordersLoading.style.display = 'none';

        if (!Array.isArray(data) || data.length === 0) {
            noOrders.style.display = 'block';
            return;
        }

        ordersTable.style.display = 'table';
        ordersBody.innerHTML = data.slice(0, 10).map(order => `
            <tr>
                <td><strong>${order.symbol}</strong></td>
                <td class="${order.side === 'buy' ? 'positive' : 'negative'}">${order.side.toUpperCase()}</td>
                <td>${order.qty}</td>
                <td>${order.type}</td>
                <td>${order.status}</td>
                <td>${formatDate(order.created_at)}</td>
            </tr>
        `).join('');
    } catch (err) {
        ordersLoading.style.display = 'none';
        ordersError.style.display = 'block';
        ordersError.textContent = `Error: ${err.message}`;
    }
}

// Handle order form submission
if (orderForm) {
    orderForm.addEventListener('submit', async (e) => {
        e.preventDefault();

        orderSuccess.style.display = 'none';
        orderError.style.display = 'none';
        submitBtn.disabled = true;
        submitBtn.textContent = 'Placing Order...';

        const symbol = document.getElementById('symbol').value.toUpperCase();
        const side = document.getElementById('side').value;
        const qty = parseFloat(document.getElementById('qty').value);
        const orderType = document.getElementById('order-type').value;
        const limitPrice = document.getElementById('limit-price').value;
        const timeInForce = document.getElementById('time-in-force').value;

        const orderData = {
            symbol,
            qty,
            side,
            order_type: orderType,
            time_in_force: timeInForce
        };

        if (orderType === 'limit' && limitPrice) {
            orderData.limit_price = parseFloat(limitPrice);
        }

        try {
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

            if (!response.ok) {
                throw new Error(data.message || data.error || 'Failed to place order');
            }

            orderSuccess.style.display = 'block';
            orderSuccess.textContent = `Order placed successfully! Order ID: ${data.id || 'N/A'}`;
            orderForm.reset();

            // Refresh orders and account
            setTimeout(() => {
                fetchOrders();
                fetchAccount();
            }, 1000);

        } catch (err) {
            orderError.style.display = 'block';
            orderError.textContent = `Error: ${err.message}`;
        } finally {
            submitBtn.disabled = false;
            submitBtn.textContent = 'Place Order';
        }
    });
}

// Show/hide limit price based on order type
const orderTypeSelect = document.getElementById('order-type');
if (orderTypeSelect) {
    orderTypeSelect.addEventListener('change', (e) => {
        const limitPriceInput = document.getElementById('limit-price');
        if (e.target.value === 'limit') {
            limitPriceInput.required = true;
            limitPriceInput.placeholder = 'Required';
        } else {
            limitPriceInput.required = false;
            limitPriceInput.placeholder = 'Optional';
        }
    });
}

// Initial load
document.addEventListener('DOMContentLoaded', () => {
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
    }
}, 30000);