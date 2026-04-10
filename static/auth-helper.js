// Shared Authentication Utilities

async function performLogout(apiBase, token) {
    try {
        await fetch(`${apiBase}/api/logout`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${token}`
            }
        });
    } catch (err) {
        console.error('Logout request failed:', err);
    } finally {
        localStorage.removeItem('token');
        localStorage.removeItem('username');
        window.location.href = '/login.html';
    }
}

// Export if using modules, but here we just use global scope for simplicity in static HTML
window.performLogout = performLogout;
