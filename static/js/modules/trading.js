import { API_BASE, fetchWithLogging, devLog, devError } from './utils.js';
import { getAuthHeaders } from './auth.js';
import { formatDate, formatCurrency } from './ui.js';

export async function fetchOrders() {
    const ordersLoading = document.getElementById('orders-loading');
    const ordersTable = document.getElementById('orders-table');
    const ordersBody = document.getElementById('orders-body');
    const noOrders = document.getElementById('no-orders');
    const ordersError = document.getElementById('orders-error');
    const cancelAllBtn = document.getElementById('cancel-all-orders-btn');
    const cancelSelectedBtn = document.getElementById('cancel-selected-btn');
    const selectAllCheckbox = document.getElementById('select-all-orders');

    if (!ordersLoading) return;

    ordersLoading.style.display = 'block';
    ordersTable.style.display = 'none';
    noOrders.style.display = 'none';
    ordersError.style.display = 'none';

    try {
        const response = await fetchWithLogging(`${API_BASE}/api/orders`, { headers: getAuthHeaders() });
        const data = response._body;
        if (!response.ok) throw new Error(data.message || data.error || 'Failed to fetch orders');

        ordersLoading.style.display = 'none';
        if (!Array.isArray(data) || data.length === 0) {
            noOrders.style.display = 'block';
            if (cancelAllBtn) cancelAllBtn.style.display = 'none';
            if (cancelSelectedBtn) cancelSelectedBtn.style.display = 'none';
            return;
        }

        ordersTable.style.display = 'table';
        const openOrders = data.filter(order => order.status === 'open' || order.status === 'pending_new');
        if (cancelAllBtn) cancelAllBtn.style.display = openOrders.length > 0 ? 'block' : 'none';

        ordersBody.innerHTML = data.slice(0, 20).map(order => {
            const isOpen = order.status === 'open' || order.status === 'pending_new';
            return `
            <tr data-order-id="${order.id}">
                <td>${isOpen ? `<input type="checkbox" class="order-checkbox" data-order-id="${order.id}">` : ''}</td>
                <td><strong>${order.symbol}</strong></td>
                <td class="${order.side === 'buy' ? 'positive' : 'negative'}">${order.side.toUpperCase()}</td>
                <td>${order.qty}</td>
                <td>${order.type}</td>
                <td><span class="status-${order.status}">${order.status}</span></td>
                <td>${formatDate(order.created_at)}</td>
                <td>
                    <div class="order-actions-cell">
                        <button class="btn-refresh btn-small btn-details" data-id="${order.id}">Details</button>
                        ${isOpen ? `<button class="btn-cancel btn-small btn-cancel-order" data-id="${order.id}">Cancel</button>` : ''}
                    </div>
                </td>
            </tr>`;
        }).join('');

        if (selectAllCheckbox) selectAllCheckbox.checked = false;
        updateCancelSelectedButton();
    } catch (err) {
        ordersLoading.style.display = 'none';
        ordersError.style.display = 'block';
        ordersError.textContent = `Error: ${err.message}`;
        if (cancelAllBtn) cancelAllBtn.style.display = 'none';
        if (cancelSelectedBtn) cancelSelectedBtn.style.display = 'none';
    }
}

export function updateCancelSelectedButton() {
    const cancelSelectedBtn = document.getElementById('cancel-selected-btn');
    const selectAllCheckbox = document.getElementById('select-all-orders');
    const checkboxes = document.querySelectorAll('.order-checkbox:checked');
    const allCheckboxes = document.querySelectorAll('.order-checkbox');

    if (cancelSelectedBtn) {
        cancelSelectedBtn.style.display = checkboxes.length > 0 ? 'block' : 'none';
        if (checkboxes.length > 0) cancelSelectedBtn.textContent = `Cancel Selected (${checkboxes.length})`;
    }
    if (selectAllCheckbox && allCheckboxes.length > 0) {
        selectAllCheckbox.checked = checkboxes.length === allCheckboxes.length;
    }
}

export async function cancelOrder(orderId) {
    if (!confirm('Are you sure you want to cancel this order?')) return;
    try {
        const response = await fetchWithLogging(`${API_BASE}/api/orders/${orderId}`, {
            method: 'DELETE',
            headers: getAuthHeaders()
        });
        const data = response._body;
        if (!response.ok) throw new Error(data.error || data.message || 'Failed to cancel order');
        alert('Order cancelled successfully!');
        fetchOrders();
    } catch (err) {
        alert(`Error: ${err.message}`);
    }
}

export async function viewOrderDetails(orderId) {
    const modal = document.getElementById('order-details-modal');
    const content = document.getElementById('order-details-content');
    if (modal) modal.style.display = 'block';
    if (content) content.innerHTML = '<div class="loading">Loading details...</div>';
    try {
        const response = await fetchWithLogging(`${API_BASE}/api/orders/${orderId}`, { headers: getAuthHeaders() });
        const order = response._body;
        if (!response.ok) throw new Error(order.error || 'Failed to fetch order details');
        content.innerHTML = `<div class="details-grid">
            <div class="detail-item"><span class="label">Order ID</span><span class="value" style="font-size: 0.8rem;">${order.id}</span></div>
            <div class="detail-item"><span class="label">Symbol</span><span class="value">${order.symbol}</span></div>
            <div class="detail-item"><span class="label">Side</span><span class="value ${order.side === 'buy' ? 'positive' : 'negative'}">${order.side.toUpperCase()}</span></div>
            <div class="detail-item"><span class="label">Status</span><span class="value"><span class="status-${order.status}">${order.status}</span></span></div>
            <div class="detail-item"><span class="label">Quantity</span><span class="value">${order.qty}</span></div>
            <div class="detail-item"><span class="label">Filled Quantity</span><span class="value">${order.filled_qty || 0}</span></div>
            <div class="detail-item"><span class="label">Order Type</span><span class="value">${order.type}</span></div>
            <div class="detail-item"><span class="label">Time in Force</span><span class="value">${order.time_in_force.toUpperCase()}</span></div>
            <div class="detail-item"><span class="label">Limit Price</span><span class="value">${order.limit_price ? formatCurrency(order.limit_price) : '-'}</span></div>
            <div class="detail-item"><span class="label">Avg Fill Price</span><span class="value">${order.filled_avg_price ? formatCurrency(order.filled_avg_price) : '-'}</span></div>
            <div class="detail-item"><span class="label">Created At</span><span class="value">${formatDate(order.created_at)}</span></div>
            <div class="detail-item"><span class="label">Updated At</span><span class="value">${formatDate(order.updated_at)}</span></div>
        </div>`;
    } catch (err) {
        if (content) content.innerHTML = `<div class="error-message">Error: ${err.message}</div>`;
    }
}
