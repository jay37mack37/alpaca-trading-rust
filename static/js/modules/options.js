import { API_BASE, fetchWithLogging } from './utils.js';
import { getAuthHeaders } from './auth.js';
import { formatCurrency } from './ui.js';

let optionsData = null;
let optionsCanvas = null;
let selectedOptionInfo = null;

function getNextFriday() {
    const today = new Date();
    const day = today.getDay();
    const diff = (day <= 5) ? (5 - day) : (12 - day);
    const nextFriday = new Date(today.getTime() + diff * 24 * 60 * 60 * 1000);
    return nextFriday.toISOString().split('T')[0];
}

export function selectOptionForOrder(symbol, type, strike, price) {
    const optionToggle = document.querySelector('.toggle-btn[data-class="option"]');
    if (optionToggle) optionToggle.click();

    document.getElementById('option-symbol').value = symbol;
    document.getElementById('option-type-hidden').value = type;
    document.getElementById('strike-price').value = strike;
    document.getElementById('option-limit-price').value = price;
    document.getElementById('option-order-type').value = 'limit';

    const event = new Event('change');
    document.getElementById('option-order-type').dispatchEvent(event);

    document.querySelectorAll('.option-type-btn').forEach(btn => {
        btn.classList.remove('active');
        if (btn.dataset.type === type) btn.classList.add('active');
    });

    document.getElementById('new-order-section').scrollIntoView({ behavior: 'smooth' });
}

function showStrikeDetails(strikeData) {
    if (!optionsData || !strikeData || !selectedOptionInfo) return;

    const stockPrice = optionsData.stockPrice;
    const strike = strikeData.strike;

    const callAsk = strikeData.call.ask.toFixed(2);
    const callBid = strikeData.call.bid.toFixed(2);
    const putAsk = strikeData.put.ask.toFixed(2);
    const putBid = strikeData.put.bid.toFixed(2);

    const callMoneyness = strike < stockPrice ? 'ITM' : (strike > stockPrice ? 'OTM' : 'ATM');
    const putMoneyness = strike > stockPrice ? 'ITM' : (strike < stockPrice ? 'OTM' : 'ATM');

    // Attach to window so onclick in template works, or better use event delegation
    window.selectOptionForOrder = selectOptionForOrder;

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
    `;
}

function drawOptionsChart() {
    if (!optionsData || !optionsCanvas || !optionsData.strikes || optionsData.strikes.length === 0) return;

    const ctx = optionsCanvas.getContext('2d');
    const width = optionsCanvas.width;
    const height = optionsCanvas.height;

    ctx.clearRect(0, 0, width, height);

    const stockPrice = optionsData.stockPrice;
    const strikes = optionsData.strikes.map(s => s.strike);
    const minStrike = Math.min(...strikes);
    const maxStrike = Math.max(...strikes);
    const range = maxStrike - minStrike;
    const strikeIncrement = range / 10;

    const margin = { top: 40, right: 60, bottom: 50, left: 80 };
    const chartWidth = width - margin.left - margin.right;
    const chartHeight = height - margin.top - margin.bottom;

    const xScale = (strike) => margin.left + ((strike - minStrike) / (maxStrike - minStrike)) * chartWidth;

    ctx.fillStyle = 'rgba(0, 0, 0, 0.3)';
    ctx.fillRect(margin.left, margin.top, chartWidth, chartHeight);

    ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
    for (let strike = minStrike; strike <= maxStrike; strike += strikeIncrement * 2) {
        const x = xScale(strike);
        ctx.beginPath(); ctx.moveTo(x, margin.top); ctx.lineTo(x, margin.top + chartHeight); ctx.stroke();
    }

    ctx.fillStyle = '#888';
    ctx.font = '12px Arial';
    ctx.textAlign = 'center';
    for (let strike = minStrike; strike <= maxStrike; strike += strikeIncrement * 2) {
        ctx.fillText(strike.toFixed(0), xScale(strike), margin.top + chartHeight + 20);
    }

    const stockX = xScale(stockPrice);
    ctx.strokeStyle = '#ffd700';
    ctx.lineWidth = 2;
    ctx.setLineDash([5, 5]);
    ctx.beginPath(); ctx.moveTo(stockX, margin.top); ctx.lineTo(stockX, margin.top + chartHeight); ctx.stroke();
    ctx.setLineDash([]);
    ctx.fillStyle = '#ffd700'; ctx.font = 'bold 12px Arial';
    ctx.fillText(`$${stockPrice.toFixed(2)}`, stockX, margin.top - 10);

    ctx.fillStyle = 'rgba(0, 255, 136, 0.1)';
    ctx.fillRect(margin.left, margin.top, stockX - margin.left, chartHeight);
    ctx.fillStyle = 'rgba(255, 68, 68, 0.1)';
    ctx.fillRect(stockX, margin.top, margin.left + chartWidth - stockX, chartHeight);

    const allPrices = optionsData.strikes.flatMap(s => [s.call.ask, s.put.ask]);
    const maxPrice = Math.max(...allPrices, 1);
    const yScale = (price) => margin.top + chartHeight - (price / maxPrice) * chartHeight;

    const sortedStrikes = [...optionsData.strikes].sort((a, b) => a.strike - b.strike);

    const drawCurve = (prop, sub, color, width) => {
        ctx.strokeStyle = color; ctx.lineWidth = width; ctx.beginPath();
        sortedStrikes.forEach((s, i) => {
            const y = yScale(s[prop][sub]);
            if (i === 0) ctx.moveTo(xScale(s.strike), y);
            else ctx.lineTo(xScale(s.strike), y);
        });
        ctx.stroke();
    };

    drawCurve('call', 'ask', '#00ff88', 3);
    drawCurve('call', 'bid', '#88ffaa', 1);
    drawCurve('put', 'ask', '#ff4444', 3);
    drawCurve('put', 'bid', '#ff8888', 1);

    ctx.fillStyle = '#888'; ctx.font = '12px Arial'; ctx.textAlign = 'right';
    for (let i = 0; i <= 4; i++) {
        const price = (maxPrice / 4) * i;
        ctx.fillText(`$${price.toFixed(0)}`, margin.left - 10, yScale(price) + 4);
    }

    ctx.fillStyle = '#fff'; ctx.font = 'bold 14px Arial'; ctx.textAlign = 'center';
    const title = optionsData.expirationFormatted ? `${optionsData.symbol} Options Chain - ${optionsData.expirationFormatted}` : `${optionsData.symbol} Options Chain`;
    ctx.fillText(title, width / 2, 20);

    optionsCanvas.onclick = (e) => {
        const rect = optionsCanvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const strike = minStrike + ((x - margin.left) / chartWidth) * (maxStrike - minStrike);
        let closest = sortedStrikes[0];
        let minDist = Math.abs(strike - closest.strike);
        sortedStrikes.forEach(s => {
            const dist = Math.abs(strike - s.strike);
            if (dist < minDist) { minDist = dist; closest = s; }
        });
        showStrikeDetails(closest);
    };
}

export function initOptionsChain() {
    const loadOptionsBtn = document.getElementById('load-options-btn');
    const optionsSymbolInput = document.getElementById('options-symbol');
    const optionsExpirationInput = document.getElementById('options-expiration');
    const optionsLoading = document.getElementById('options-loading');
    const optionsChartContainer = document.getElementById('options-chart-container');
    const optionsError = document.getElementById('options-error');
    const stockPriceValue = document.getElementById('stock-price-value');
    optionsCanvas = document.getElementById('options-chart');
    selectedOptionInfo = document.getElementById('selected-option-info');

    if (optionsExpirationInput && !optionsExpirationInput.value) optionsExpirationInput.value = getNextFriday();
    if (!loadOptionsBtn) return;

    loadOptionsBtn.addEventListener('click', async () => {
        const symbol = optionsSymbolInput ? optionsSymbolInput.value.toUpperCase() : '';
        const expiration = optionsExpirationInput ? optionsExpirationInput.value : '';
        if (!symbol || !expiration) { alert('Please enter symbol and expiration'); return; }

        loadOptionsBtn.disabled = true;
        if (optionsLoading) optionsLoading.style.display = 'block';
        if (optionsChartContainer) optionsChartContainer.style.display = 'none';

        try {
            const response = await fetchWithLogging(`${API_BASE}/api/option-chain/${symbol}`, { headers: getAuthHeaders() });
            if (!response.ok) throw new Error((response._body).error || 'Failed to load options data');
            const chainData = response._body;

            const [eY, eM, eD] = expiration.split('-').map(Number);
            const expDate = new Date(eY, eM - 1, eD);
            optionsData = {
                symbol, stockPrice: chainData.underlying_price, strikes: chainData.strikes, expiration,
                expirationFormatted: expDate.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
            };
            if (stockPriceValue) stockPriceValue.textContent = formatCurrency(chainData.underlying_price);
            if (optionsLoading) optionsLoading.style.display = 'none';
            if (optionsChartContainer) optionsChartContainer.style.display = 'block';
            drawOptionsChart();
        } catch (err) {
            if (optionsLoading) optionsLoading.style.display = 'none';
            if (optionsError) { optionsError.style.display = 'block'; optionsError.textContent = `Error: ${err.message}`; }
        } finally {
            loadOptionsBtn.disabled = false;
        }
    });
}
