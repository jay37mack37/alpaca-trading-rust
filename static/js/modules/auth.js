import { API_BASE, fetchWithLogging } from './utils.js';

export function getAuthHeaders() {
    const token = localStorage.getItem('token');
    return {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`
    };
}

export function checkAuth() {
    const token = localStorage.getItem('token');
    if (!token) {
        window.location.href = '/login.html';
        return false;
    }

    // Verify token
    fetchWithLogging(`${API_BASE}/api/verify`, {
        headers: { 'Authorization': `Bearer ${token}` }
    }).then(res => {
        if (!res.ok) {
            localStorage.removeItem('token');
            window.location.href = '/login.html';
        }
    }).catch(() => {
        // Silent fail, rely on next API call
    });

    const username = localStorage.getItem('username');
    const userDisplay = document.getElementById('user-display');
    if (userDisplay && username) {
        userDisplay.textContent = `👤 ${username}`;
    }

    const authCheck = document.getElementById('auth-check');
    if (authCheck) authCheck.style.display = 'none';

    return true;
}

export async function performLogout() {
    const token = localStorage.getItem('token');
    try {
        await fetch(`${API_BASE}/api/logout`, {
            method: 'POST',
            headers: { 'Authorization': `Bearer ${token}` }
        });
    } catch (e) {
        console.error('Logout failed', e);
    }
    localStorage.removeItem('token');
    localStorage.removeItem('username');
    window.location.href = '/login.html';
}
