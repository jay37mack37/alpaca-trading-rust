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

// Add event listeners for order management buttons
document.addEventListener('DOMContentLoaded', () => {
    const cancelAllBtn = document.getElementById('cancel-all-orders-btn');
    if (cancelAllBtn) {
        cancelAllBtn.addEventListener('click', cancelAllOrders);
    }

    const cancelSelectedBtn = document.getElementById('cancel-selected-btn');
    if (cancelSelectedBtn) {
        cancelSelectedBtn.addEventListener('click', cancelSelectedOrders);
    }

    const selectAllCheckbox = document.getElementById('select-all-orders');
    if (selectAllCheckbox) {
        selectAllCheckbox.addEventListener('change', toggleSelectAllOrders);
    }
});

// Asset class toggle
let currentAssetClass = 'stock';

document.querySelectorAll('.toggle-btn').forEach(btn => {
    btn.addEventListener('click', () => {
        document.querySelectorAll('.toggle-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        currentAssetClass = btn.dataset.class;

        // Toggle visibility
        document.getElementById('stock-fields').style.display = currentAssetClass === 'stock' ? 'block' : 'none';
        document.getElementById('option-fields').style.display = currentAssetClass === 'option' ? 'block' : 'none';

        // When switching to option, default to CALL
        if (currentAssetClass === 'option') {
            document.querySelectorAll('.option-type-btn').forEach(b => b.classList.remove('active'));
            const callBtn = document.querySelector('.option-type-btn.call-btn');
            if (callBtn) {
                callBtn.classList.add('active');
                selectedOptionType = 'call';
                document.getElementById('option-type-hidden').value = 'call';
            }
        }
    });
});

// Option type toggle
let selectedOptionType = null;
document.querySelectorAll('.option-type-btn').forEach(btn => {
    btn.addEventListener('click', () => {
        document.querySelectorAll('.option-type-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        selectedOptionType = btn.dataset.type;
        document.getElementById('option-type-hidden').value = selectedOptionType;
    });
});

// Set default expiration date to next Friday
function getNextFriday() {
    const today = new Date();
    const dayOfWeek = today.getDay();
    const daysUntilFriday = (5 - dayOfWeek + 7) % 7 || 7;
    const nextFriday = new Date(today);
    nextFriday.setDate(today.getDate() + daysUntilFriday);
    return nextFriday.toISOString().split('T')[0];
}
document.getElementById('expiration-date').value = getNextFriday();

// Handle order form submission
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

                orderData = {
                    symbol,
                    qty,
                    side,
                    order_type: orderType,
                    time_in_force: timeInForce
                };

                if (orderType === 'limit' && limitPrice) {
                    orderData.limit_price = parseFloat(limitPrice);
                }
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

                if (!optionType) {
                    throw new Error('Please select Call or Put');
                }

                // Build OCC symbol: SPY   240408C00500000
                // Format: Symbol (6 chars) + YYMMDD + C/P + Strike (8 chars, 3 decimal places)
                const expDate = new Date(expiration);
                const yy = expDate.getFullYear().toString().slice(-2);
                const mm = (expDate.getMonth() + 1).toString().padStart(2, '0');
                const dd = expDate.getDate().toString().padStart(2, '0');
                const cp = optionType === 'call' ? 'C' : 'P';
                const strikeStr = (strike * 1000).toFixed(0).padStart(8, '0');
                const symbol = underlying.padEnd(6, ' ') + yy + mm + dd + cp + strikeStr;

                orderData = {
                    symbol: symbol.replace(/\s/g, ''),
                    qty,
                    side,
                    order_type: orderType,
                    time_in_force: timeInForce,
                    asset_class: 'us_option'
                };

                if (orderType === 'limit' && limitPrice) {
                    orderData.limit_price = parseFloat(limitPrice);
                }
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

            if (!response.ok) {
                throw new Error(data.message || data.error || 'Failed to place order');
            }

            orderSuccess.style.display = 'block';
            orderSuccess.textContent = `Order placed successfully! Order ID: ${data.id || 'N/A'}`;
            orderForm.reset();
            document.getElementById('symbol').value = 'SPY';
            document.getElementById('option-symbol').value = 'SPY';
            document.getElementById('expiration-date').value = getNextFriday();

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

// Show/hide limit price based on order type (stock)
const orderTypeSelect = document.getElementById('order-type');
const fillPriceBtn = document.getElementById('fill-price-btn');
const limitPriceInput = document.getElementById('limit-price');
const symbolInput = document.getElementById('symbol');

if (orderTypeSelect) {
    orderTypeSelect.addEventListener('change', (e) => {
        if (e.target.value === 'limit') {
            limitPriceInput.required = true;
            limitPriceInput.placeholder = 'Required';
            fillPriceBtn.style.display = 'block';
        } else {
            limitPriceInput.required = false;
            limitPriceInput.placeholder = 'Optional';
            fillPriceBtn.style.display = 'none';
        }
    });
}

// Fill market price button
if (fillPriceBtn) {
    fillPriceBtn.addEventListener('click', async () => {
        const symbol = symbolInput.value.toUpperCase();
        if (!symbol) {
            alert('Please enter a symbol first');
            return;
        }

        fillPriceBtn.disabled = true;
        fillPriceBtn.textContent = 'Loading...';

        try {
            const response = await fetch(`${API_BASE}/api/price/${symbol}`, {
                headers: getAuthHeaders()
            });

            if (response.status === 401) {
                localStorage.removeItem('token');
                window.location.href = '/login.html';
                return;
            }

            const data = await response.json();

            if (!response.ok) {
                throw new Error(data.error || 'Failed to get price');
            }

            // Alpaca returns quote with:
            // - bp: bid price
            // - ap: ask price (may be 0 when market closed)
            // - For buy orders, use ask price; for sell orders, use bid price
            const side = document.getElementById('side').value;
            let price;

            if (data.quote) {
                const askPrice = data.quote.ap || data.quote.bp; // fallback to bid if ask is 0
                const bidPrice = data.quote.bp;
                price = side === 'buy' ? askPrice : bidPrice;
            } else {
                throw new Error('Price data not available');
            }

            if (!price || price === 0) {
                throw new Error('Price not available (market may be closed)');
            }

            limitPriceInput.value = parseFloat(price).toFixed(2);
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

if (optionOrderTypeSelect) {
    optionOrderTypeSelect.addEventListener('change', (e) => {
        if (e.target.value === 'limit') {
            optionLimitPriceInput.required = true;
            optionLimitPriceInput.placeholder = 'Required';
            fillOptionPriceBtn.style.display = 'block';
        } else {
            optionLimitPriceInput.required = false;
            optionLimitPriceInput.placeholder = 'Optional';
            fillOptionPriceBtn.style.display = 'none';
        }
    });
}

// Fill strike button (uses selected call/put)
const fillStrikeBtn = document.getElementById('fill-strike-btn');
const strikePriceInput = document.getElementById('strike-price');

async function getOptionStrikes(underlying) {
    const response = await fetch(`${API_BASE}/api/option-quote/${underlying}`, {
        headers: getAuthHeaders()
    });

    if (response.status === 401) {
        localStorage.removeItem('token');
        window.location.href = '/login.html';
        return null;
    }

    const data = await response.json();

    if (!response.ok) {
        throw new Error(data.error || 'Failed to get option strikes');
    }

    return data;
}

if (fillStrikeBtn) {
    fillStrikeBtn.addEventListener('click', async () => {
        const symbol = optionSymbolInput.value.toUpperCase();
        if (!symbol) {
            alert('Please enter an underlying symbol first');
            return;
        }

        if (!selectedOptionType) {
            alert('Please select Call or Put first');
            return;
        }

        fillStrikeBtn.disabled = true;
        fillStrikeBtn.textContent = 'Loading...';

        try {
            const data = await getOptionStrikes(symbol);
            if (data) {
                const strike = selectedOptionType === 'call' ? data.call_strike : data.put_strike;
                if (strike) {
                    strikePriceInput.value = strike;
                } else {
                    alert(`No ${selectedOptionType} strikes available`);
                }
            } else {
                alert('No strikes available');
            }
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
        const underlying = optionSymbolInput.value.toUpperCase();
        const strike = strikePriceInput.value;
        const expiration = document.getElementById('expiration-date').value;
        const optionType = document.getElementById('option-type-hidden').value;

        if (!underlying || !strike || !expiration || !optionType) {
            alert('Please fill in all option details first');
            return;
        }

        fillOptionPriceBtn.disabled = true;
        fillOptionPriceBtn.textContent = 'Loading...';

        try {
            // Build OCC symbol for the option
            const expDate = new Date(expiration);
            const yy = expDate.getFullYear().toString().slice(-2);
            const mm = (expDate.getMonth() + 1).toString().padStart(2, '0');
            const dd = expDate.getDate().toString().padStart(2, '0');
            const cp = optionType === 'call' ? 'C' : 'P';
            const strikeStr = (parseFloat(strike) * 1000).toFixed(0).padStart(8, '0');
            const optionSymbol = underlying.padEnd(6, ' ') + yy + mm + dd + cp + strikeStr;

            const response = await fetch(`${API_BASE}/api/option-strikes/${optionSymbol.replace(/\s/g, '')}`, {
                headers: getAuthHeaders()
            });

            if (response.status === 401) {
                localStorage.removeItem('token');
                window.location.href = '/login.html';
                return;
            }

            const data = await response.json();

            if (!response.ok) {
                throw new Error(data.error || 'Failed to get option price');
            }

            // For options, use bid/ask similar to stocks
            let price;
            const side = document.getElementById('option-side').value;

            if (data.quote) {
                const askPrice = data.quote.ap || data.quote.bp;
                const bidPrice = data.quote.bp;
                // For buy orders use ask, for sell orders use bid
                price = side.includes('buy') ? askPrice : bidPrice;
            } else {
                throw new Error('Price data not available');
            }

            if (!price || price === 0) {
                throw new Error('Price not available (market may be closed)');
            }

            optionLimitPriceInput.value = parseFloat(price).toFixed(2);
        } catch (err) {
            alert(`Error: ${err.message}`);
        } finally {
            fillOptionPriceBtn.disabled = false;
            fillOptionPriceBtn.textContent = 'Fill Market Price';
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

// Options Chain Chart
let optionsData = null;
let selectedStrike = null;

const loadOptionsBtn = document.getElementById('load-options-btn');
const optionsSymbolInput = document.getElementById('options-symbol');
const optionsExpirationInput = document.getElementById('options-expiration');
const optionsLoading = document.getElementById('options-loading');
const optionsChartContainer = document.getElementById('options-chart-container');
const optionsError = document.getElementById('options-error');
const stockPriceValue = document.getElementById('stock-price-value');
const optionsCanvas = document.getElementById('options-chart');
const selectedOptionInfo = document.getElementById('selected-option-info');

// Set default expiration date to next Friday
if (optionsExpirationInput) {
    optionsExpirationInput.value = getNextFriday();
}

if (loadOptionsBtn) {
    loadOptionsBtn.addEventListener('click', async () => {
        const symbol = optionsSymbolInput.value.toUpperCase();
        const expiration = optionsExpirationInput.value;

        if (!symbol) {
            alert('Please enter a symbol');
            return;
        }
        if (!expiration) {
            alert('Please select an expiration date');
            return;
        }

        loadOptionsBtn.disabled = true;
        loadOptionsBtn.textContent = 'Loading...';
        optionsLoading.style.display = 'block';
        optionsChartContainer.style.display = 'none';
        optionsError.style.display = 'none';

        try {
            // Get current stock price first
            const priceResponse = await fetch(`${API_BASE}/api/price/${symbol}`, {
                headers: getAuthHeaders()
            });

            if (priceResponse.status === 401) {
                localStorage.removeItem('token');
                window.location.href = '/login.html';
                return;
            }

            const priceData = await priceResponse.json();
            let stockPrice = 0;
            if (priceData.quote) {
                stockPrice = priceData.quote.ap || priceData.quote.bp || 0;
            }

            // Fetch strike data with expiration date
            const strikesResponse = await fetch(`${API_BASE}/api/option-quote/${symbol}?expiration=${expiration}`, {
                headers: getAuthHeaders()
            });

            if (!strikesResponse.ok) {
                throw new Error('Failed to load options data');
            }

            const strikesData = await strikesResponse.json();

            // Format expiration date for display
            const expDate = new Date(expiration);
            const expFormatted = expDate.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });

            // Store data for charting
            optionsData = {
                symbol: symbol,
                stockPrice: stockPrice,
                callStrike: strikesData.call_strike,
                putStrike: strikesData.put_strike,
                strikeIncrement: strikesData.strike_increment,
                expiration: expiration,
                expirationFormatted: expFormatted
            };

            // Display data
            stockPriceValue.textContent = formatCurrency(stockPrice);
            optionsLoading.style.display = 'none';
            optionsChartContainer.style.display = 'block';

            // Draw the chart
            drawOptionsChart();

        } catch (err) {
            optionsLoading.style.display = 'none';
            optionsError.style.display = 'block';
            optionsError.textContent = `Error: ${err.message}`;
        } finally {
            loadOptionsBtn.disabled = false;
            loadOptionsBtn.textContent = 'Load Options';
        }
    });
}

function drawOptionsChart() {
    if (!optionsData || !optionsCanvas) return;

    const ctx = optionsCanvas.getContext('2d');
    const width = optionsCanvas.width;
    const height = optionsCanvas.height;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    const stockPrice = optionsData.stockPrice;
    const strikeIncrement = optionsData.strikeIncrement || 5;

    // Calculate strikes range (±15 strikes from current price)
    const numStrikes = 15;
    const baseStrike = Math.floor(stockPrice / strikeIncrement) * strikeIncrement;
    const minStrike = baseStrike - (numStrikes * strikeIncrement);
    const maxStrike = baseStrike + (numStrikes * strikeIncrement);

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

    // Draw simulated option prices (ITM options have intrinsic value)
    // Call price increases as strike decreases (ITM)
    // Put price increases as strike increases (ITM)

    const maxPrice = stockPrice * 0.15; // Max expected option price

    const yScale = (price) => {
        return margin.top + chartHeight - (price / maxPrice) * chartHeight;
    };

    // Draw call price curve (ask)
    ctx.strokeStyle = '#00ff88';
    ctx.lineWidth = 3;
    ctx.beginPath();
    for (let strike = minStrike; strike <= maxStrike; strike += strikeIncrement / 2) {
        const x = xScale(strike);
        // Simulate call price: higher when strike is lower (ITM)
        const intrinsic = Math.max(0, stockPrice - strike);
        const timeValue = Math.max(0.5, (stockPrice * 0.02) * (1 - Math.abs(strike - stockPrice) / stockPrice));
        const askPrice = intrinsic + timeValue;
        const y = yScale(askPrice);
        if (strike === minStrike) {
            ctx.moveTo(x, y);
        } else {
            ctx.lineTo(x, y);
        }
    }
    ctx.stroke();

    // Draw call price curve (bid) - slightly lower
    ctx.strokeStyle = '#88ffaa';
    ctx.lineWidth = 2;
    ctx.beginPath();
    for (let strike = minStrike; strike <= maxStrike; strike += strikeIncrement / 2) {
        const x = xScale(strike);
        const intrinsic = Math.max(0, stockPrice - strike);
        const timeValue = Math.max(0.5, (stockPrice * 0.02) * (1 - Math.abs(strike - stockPrice) / stockPrice));
        const bidPrice = intrinsic + timeValue - 0.1;
        const y = yScale(bidPrice);
        if (strike === minStrike) {
            ctx.moveTo(x, y);
        } else {
            ctx.lineTo(x, y);
        }
    }
    ctx.stroke();

    // Draw put price curve (ask)
    ctx.strokeStyle = '#ff4444';
    ctx.lineWidth = 3;
    ctx.beginPath();
    for (let strike = minStrike; strike <= maxStrike; strike += strikeIncrement / 2) {
        const x = xScale(strike);
        // Simulate put price: higher when strike is higher (ITM)
        const intrinsic = Math.max(0, strike - stockPrice);
        const timeValue = Math.max(0.5, (stockPrice * 0.02) * (1 - Math.abs(strike - stockPrice) / stockPrice));
        const askPrice = intrinsic + timeValue;
        const y = yScale(askPrice);
        if (strike === minStrike) {
            ctx.moveTo(x, y);
        } else {
            ctx.lineTo(x, y);
        }
    }
    ctx.stroke();

    // Draw put price curve (bid)
    ctx.strokeStyle = '#ff8888';
    ctx.lineWidth = 2;
    ctx.beginPath();
    for (let strike = minStrike; strike <= maxStrike; strike += strikeIncrement / 2) {
        const x = xScale(strike);
        const intrinsic = Math.max(0, strike - stockPrice);
        const timeValue = Math.max(0.5, (stockPrice * 0.02) * (1 - Math.abs(strike - stockPrice) / stockPrice));
        const bidPrice = intrinsic + timeValue - 0.1;
        const y = yScale(bidPrice);
        if (strike === minStrike) {
            ctx.moveTo(x, y);
        } else {
            ctx.lineTo(x, y);
        }
    }
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
        const roundedStrike = Math.round(strike / strikeIncrement) * strikeIncrement;

        showStrikeDetails(roundedStrike);
    };
}

function showStrikeDetails(strike) {
    if (!optionsData) return;

    const stockPrice = optionsData.stockPrice;
    const strikeIncrement = optionsData.strikeIncrement || 5;

    // Calculate approximate prices
    const callIntrinsic = Math.max(0, stockPrice - strike);
    const putIntrinsic = Math.max(0, strike - stockPrice);
    const timeValue = Math.max(0.5, (stockPrice * 0.02) * (1 - Math.abs(strike - stockPrice) / stockPrice));

    const callAsk = (callIntrinsic + timeValue).toFixed(2);
    const callBid = (callIntrinsic + timeValue - 0.1).toFixed(2);
    const putAsk = (putIntrinsic + timeValue).toFixed(2);
    const putBid = (putIntrinsic + timeValue - 0.1).toFixed(2);

    const callMoneyness = strike < stockPrice ? 'ITM' : (strike > stockPrice ? 'OTM' : 'ATM');
    const putMoneyness = strike > stockPrice ? 'ITM' : (strike < stockPrice ? 'OTM' : 'ATM');

    selectedOptionInfo.innerHTML = `
        <div class="option-detail-grid">
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