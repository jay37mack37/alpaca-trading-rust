<script lang="ts">
    import { onMount } from 'svelte';
    import { createChart, ColorType } from 'lightweight-charts';
    import { api } from '../lib/api';

    export let strategyId: string;

    let performanceData: any = null;
    
    // Chart instances
    let equityContainer: HTMLElement;
    let drawdownContainer: HTMLElement;
    let equityChart: any;
    let drawdownChart: any;
    let equitySeries: any;
    let drawdownSeries: any;

    async function loadPerformance() {
        try {
            const resp = await api.get(`/analytics/performance?strategy_id=${strategyId}`);
            if (resp.success) {
                performanceData = resp.data;
                updateCharts();
            }
        } catch (e) {
            console.error("Failed to load performance", e);
        }
    }

    function initCharts() {
        // Common options
        const commonOptions = {
            layout: {
                background: { type: ColorType.Solid, color: 'transparent' },
                textColor: '#94a3b8',
                fontFamily: 'JetBrains Mono',
            },
            grid: {
                vertLines: { color: 'rgba(255, 255, 255, 0.03)' },
                horzLines: { color: 'rgba(255, 255, 255, 0.03)' },
            },
            rightPriceScale: { borderVisible: false },
            timeScale: { borderVisible: false },
        };

        // Equity Curve
        equityChart = createChart(equityContainer, {
            ...commonOptions,
            width: equityContainer.clientWidth,
            height: 350,
        });
        equitySeries = equityChart.addAreaSeries({
            lineColor: '#22d3ee',
            topColor: 'rgba(34, 211, 238, 0.3)',
            bottomColor: 'rgba(34, 211, 238, 0)',
            lineWidth: 2,
        });

        // Drawdown Chart (Underwater)
        drawdownChart = createChart(drawdownContainer, {
            ...commonOptions,
            width: drawdownContainer.clientWidth,
            height: 150,
        });
        drawdownSeries = drawdownChart.addAreaSeries({
            lineColor: '#f87171',
            topColor: 'rgba(248, 113, 113, 0)',
            bottomColor: 'rgba(248, 113, 113, 0.3)',
            lineWidth: 2,
        });

        window.addEventListener('resize', () => {
            equityChart.applyOptions({ width: equityContainer.clientWidth });
            drawdownChart.applyOptions({ width: drawdownContainer.clientWidth });
        });
    }

    function updateCharts() {
        if (!performanceData) return;
        
        const equityPoints = performanceData.equity_curve.map((p: any, i: number) => ({
            time: i,
            value: p.equity
        }));
        equitySeries.setData(equityPoints);
        equityChart.timeScale().fitContent();

        // Calculate drawdown points
        let peak = 0;
        const drawdownPoints = performanceData.equity_curve.map((p: any, i: number) => {
            if (p.equity > peak) peak = p.equity;
            const dd = peak === 0 ? 0 : (p.equity - peak);
            return { time: i, value: dd };
        });
        drawdownSeries.setData(drawdownPoints);
        drawdownChart.timeScale().fitContent();
    }

    onMount(() => {
        initCharts();
        loadPerformance();
        const interval = setInterval(loadPerformance, 10000);
        return () => clearInterval(interval);
    });
</script>

<div class="performance-view space-y-6">
    <!-- Top Bento: Summary Stats -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div class="bento-stat glass-panel p-6">
            <span class="label">Total Net Alpha</span>
            <div class="value text-cyan-400 font-mono">${performanceData?.metrics.total_pnl.toFixed(2) || '0.00'}</div>
            <div class="sub-label">{(performanceData?.metrics.total_pnl / 10).toFixed(2)}% ROI</div>
        </div>
        <div class="bento-stat glass-panel p-6">
            <span class="label">Profit Factor</span>
            <div class="value font-mono">{performanceData?.metrics.profit_factor.toFixed(2) || '0.00'}</div>
            <div class="sub-label">Efficiency Ratio</div>
        </div>
        <div class="bento-stat glass-panel p-6">
            <span class="label">Win Probability</span>
            <div class="value font-mono">{(performanceData?.metrics.win_rate * 100 || 0).toFixed(1)}%</div>
            <div class="sub-label">{performanceData?.metrics.total_trades || 0} Total Cycles</div>
        </div>
        <div class="bento-stat glass-panel p-6">
            <span class="label">Execution Quality</span>
            <div class="value text-purple-400 font-mono">{(100 - (performanceData?.metrics.avg_slippage_pct || 0)).toFixed(2)}%</div>
            <div class="sub-label">Fill Optimization</div>
        </div>
    </div>

    <!-- Main Charts Area -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Equity & Drawdown (2/3 width) -->
        <div class="lg:col-span-2 space-y-4">
            <div class="glass-panel p-6">
                <header class="flex justify-between items-center mb-6">
                    <h3 class="text-sm font-bold tracking-widest uppercase text-slate-500">Live Equity Trajectory</h3>
                    <div class="flex gap-2">
                        <span class="px-2 py-1 bg-cyan-400/10 text-cyan-400 rounded text-[10px] font-bold">REALTIME</span>
                    </div>
                </header>
                <div bind:this={equityContainer} class="w-full"></div>
                
                <div class="mt-8 pt-6 border-t border-white/5">
                    <h3 class="text-sm font-bold tracking-widest uppercase text-slate-500 mb-4">Underwater Drawdown (Risk Heat)</h3>
                    <div bind:this={drawdownContainer} class="w-full"></div>
                </div>
            </div>
        </div>

        <!-- Right Side: Risk & Audit -->
        <div class="space-y-6">
            <!-- Risk Sentry Module -->
            <div class="glass-panel p-6 border-l-4 border-l-orange-500">
                <h3 class="text-xs font-bold text-slate-500 uppercase tracking-widest mb-6">Hard Sentry Telemetry</h3>
                
                <div class="space-y-8">
                    <div class="sentry-cell">
                        <div class="flex justify-between items-end mb-2">
                            <span class="text-sm font-bold text-slate-300">Daily Spam Barrier</span>
                            <span class="font-mono text-xs text-slate-500">{performanceData?.metrics.trades_today || 0} / 50</span>
                        </div>
                        <div class="energy-bar">
                            <div class="fill bg-cyan-400" style="width: {(performanceData?.metrics.trades_today / 50) * 100}%"></div>
                            <div class="glow bg-cyan-400" style="width: {(performanceData?.metrics.trades_today / 50) * 100}%"></div>
                        </div>
                    </div>

                    <div class="sentry-cell">
                        <div class="flex justify-between items-end mb-2">
                            <span class="text-sm font-bold text-slate-300">Drawdown Circuit</span>
                            <span class="font-mono text-xs text-slate-500">${Math.abs(performanceData?.metrics.daily_pnl || 0).toFixed(2)} / $50</span>
                        </div>
                        <div class="energy-bar">
                            <div class="fill bg-orange-500" style="width: {(Math.abs(performanceData?.metrics.daily_pnl || 0) / 50) * 100}%"></div>
                            <div class="glow bg-orange-500" style="width: {(Math.abs(performanceData?.metrics.daily_pnl || 0) / 50) * 100}%"></div>
                        </div>
                    </div>
                </div>

                <div class="mt-10 grid grid-cols-2 gap-4 pt-6 border-t border-white/5">
                    <div class="stat-box">
                        <div class="label text-[10px] text-slate-500 uppercase font-bold">Sharpe Ratio</div>
                        <div class="value font-mono text-xl">{performanceData?.metrics.sharpe_ratio.toFixed(2) || '0.00'}</div>
                    </div>
                    <div class="stat-box">
                        <div class="label text-[10px] text-slate-500 uppercase font-bold">Sortino</div>
                        <div class="value font-mono text-xl">{performanceData?.metrics.sortino_ratio.toFixed(2) || '0.00'}</div>
                    </div>
                </div>
            </div>

            <!-- Execution Leakage -->
            <div class="glass-panel p-6">
                <h3 class="text-xs font-bold text-slate-500 uppercase tracking-widest mb-6">Execution Leakage Audit</h3>
                <div class="space-y-4">
                    {#if performanceData?.recent_leakage.length > 0}
                        {#each performanceData.recent_leakage as leak}
                            <div class="flex justify-between items-center p-3 rounded-lg bg-white/5 border border-white/5">
                                <div>
                                    <div class="text-sm font-bold text-cyan-400">{leak.symbol}</div>
                                    <div class="text-[10px] text-slate-500">{new Date(leak.executed_at).toLocaleTimeString()}</div>
                                </div>
                                <div class="text-right">
                                    <div class="font-mono text-sm {leak.slippage_pct > 0.5 ? 'text-red-400' : 'text-slate-300'}">
                                        -{leak.slippage_pct.toFixed(3)}%
                                    </div>
                                    <div class="text-[10px] uppercase font-bold {leak.slippage_pct > 0.5 ? 'text-red-400/50' : 'text-green-400/50'}">
                                        {leak.slippage_pct > 0.5 ? 'High Leak' : 'Optimal'}
                                    </div>
                                </div>
                            </div>
                        {/each}
                    {:else}
                        <div class="text-center py-10 text-slate-600 text-xs uppercase tracking-widest">Scanning market for leakage...</div>
                    {/if}
                </div>
            </div>
        </div>
    </div>
</div>

<style>
    .performance-view {
        padding-top: 1rem;
    }

    .bento-stat .label {
        font-size: 0.65rem;
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: 0.15em;
        color: #64748b;
        display: block;
        margin-bottom: 0.5rem;
    }

    .bento-stat .value {
        font-size: 2rem;
        font-weight: 800;
        line-height: 1;
    }

    .bento-stat .sub-label {
        font-size: 0.75rem;
        color: #475569;
        margin-top: 0.5rem;
    }

    /* Sentry Energy Bars */
    .energy-bar {
        position: relative;
        height: 6px;
        background: rgba(255, 255, 255, 0.03);
        border-radius: 3px;
        overflow: hidden;
    }

    .energy-bar .fill {
        position: absolute;
        height: 100%;
        border-radius: 3px;
        z-index: 2;
        transition: width 0.8s cubic-bezier(0.4, 0, 0.2, 1);
    }

    .energy-bar .glow {
        position: absolute;
        height: 100%;
        filter: blur(8px);
        opacity: 0.5;
        z-index: 1;
        transition: width 0.8s cubic-bezier(0.4, 0, 0.2, 1);
    }

    .stat-box .label {
        margin-bottom: 0.25rem;
    }
</style>
