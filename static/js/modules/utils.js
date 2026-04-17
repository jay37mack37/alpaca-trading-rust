export const DEV_MODE = true;
const LOG_BUFFER = [];
const MAX_LOG_SIZE = 100;
export const API_BASE = window.location.origin;

export function devLog(category, message, data = null) {
    if (!DEV_MODE) return;
    const timestamp = new Date().toLocaleTimeString();
    const logEntry = { time: timestamp, category, message, data, type: 'log' };
    LOG_BUFFER.push(logEntry);
    if (LOG_BUFFER.length > MAX_LOG_SIZE) LOG_BUFFER.shift();
    console.log(`%c[${timestamp}] ${category}: ${message}`, 'color: #0066cc; font-weight: bold;', data || '');
}

export function devWarn(category, message, data = null) {
    if (!DEV_MODE) return;
    const timestamp = new Date().toLocaleTimeString();
    const logEntry = { time: timestamp, category, message, data, type: 'warn' };
    LOG_BUFFER.push(logEntry);
    if (LOG_BUFFER.length > MAX_LOG_SIZE) LOG_BUFFER.shift();
    console.warn(`%c[${timestamp}] ${category}: ${message}`, 'color: #ff9900; font-weight: bold;', data || '');
}

export function devError(category, message, data = null) {
    if (!DEV_MODE) return;
    const timestamp = new Date().toLocaleTimeString();
    const logEntry = { time: timestamp, category, message, data, type: 'error' };
    LOG_BUFFER.push(logEntry);
    if (LOG_BUFFER.length > MAX_LOG_SIZE) LOG_BUFFER.shift();
    console.error(`%c[${timestamp}] ${category}: ${message}`, 'color: #cc0000; font-weight: bold;', data || '');
}

export async function fetchWithLogging(url, options = {}) {
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

        devLog('API', `${method} ${shortUrl} → ${response.status} (${duration}ms)`, {
            status: response.status,
            statusText: response.statusText,
            duration: `${duration}ms`,
            size: `${(size / 1024).toFixed(2)}KB`,
            body: data
        });

        if (response.status === 401 && !url.includes('/api/login') && !url.includes('/api/verify')) {
            devWarn('AUTH', 'Session expired, redirecting to login');
            localStorage.removeItem('token');
            localStorage.removeItem('username');
            window.location.href = '/login.html';
        }

        response._body = data;
        return response;
    } catch (error) {
        const duration = (performance.now() - startTime).toFixed(2);
        devError('API', `${method} ${shortUrl} failed (${duration}ms)`, error.message);
        throw error;
    }
}
