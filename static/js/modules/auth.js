import { API_BASE, fetchWithLogging, devLog, devWarn } from './utils.js';

export function checkAuth() {
    const token = localStorage.getItem('token');
    const userDisplay = document.getElementById('user-display');
    const authCheck = document.getElementById('auth-check');

    if (!token) {
        window.location.href = '/login.html';
        return false;
    }

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

    return true;
}

export function getAuthHeaders() {
    const token = localStorage.getItem('token');
    return {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`
    };
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
