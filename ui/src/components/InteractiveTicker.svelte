<script lang="ts">
  import { onMount } from "svelte";
  import {
    CandlestickSeries,
    ColorType,
    HistogramSeries,
    createChart,
    type CandlestickData,
    type HistogramData,
    type IChartApi,
    type ISeriesApi,
    type Time,
    type UTCTimestamp,
  } from "lightweight-charts";
  import type { Candle } from "../lib/types";

  export let symbol: string;
  export let candles: Candle[] = [];
  export let showVwap: boolean = false;
  export let height: number = 420;

  let container: HTMLDivElement;
  let chart: IChartApi | undefined;
  let candleSeries: ISeriesApi<"Candlestick"> | undefined;
  let volumeSeries: ISeriesApi<"Histogram"> | undefined;
  let vwapSeries: ISeriesApi<"Line"> | undefined;

  function toSeriesData(series: Candle[]): CandlestickData<Time>[] {
    return series.map((candle) => ({
      time: (Date.parse(candle.timestamp) / 1000) as UTCTimestamp,
      open: candle.open,
      high: candle.high,
      low: candle.low,
      close: candle.close,
    }));
  }

  function toVolumeData(series: Candle[]): HistogramData<Time>[] {
    return series.map((candle) => ({
      time: (Date.parse(candle.timestamp) / 1000) as UTCTimestamp,
      value: candle.volume,
      color: candle.close >= candle.open ? "#4fd18b66" : "#ff6f6f55",
    }));
  }

  function syncSeries() {
    if (!candleSeries || !volumeSeries) return;
    candleSeries.setData(toSeriesData(candles));
    volumeSeries.setData(toVolumeData(candles));

    if (showVwap && vwapSeries) {
      const vwapData = candles
        .filter((c) => c.vwap != null)
        .map((c) => ({
          time: (Date.parse(c.timestamp) / 1000) as UTCTimestamp,
          value: c.vwap as number,
        }));
      vwapSeries.setData(vwapData);
    }
  }

  function resize() {
    if (!chart || !container) return;
    chart.applyOptions({
      width: container.clientWidth,
      height,
    });
  }

  onMount(() => {
    chart = createChart(container, {
      width: container.clientWidth,
      height,
      layout: {
        background: {
          type: ColorType.Solid,
          color: "transparent",
        },
        textColor: "#dce8ff",
      },
      grid: {
        vertLines: { color: "rgba(255,255,255,0.06)" },
        horzLines: { color: "rgba(255,255,255,0.06)" },
      },
      rightPriceScale: {
        borderColor: "rgba(255,255,255,0.08)",
      },
      timeScale: {
        borderColor: "rgba(255,255,255,0.08)",
        timeVisible: true,
        secondsVisible: false,
      },
      crosshair: {
        vertLine: {
          color: "rgba(255,255,255,0.2)",
          width: 1,
        },
        horzLine: {
          color: "rgba(255,255,255,0.2)",
          width: 1,
        },
      },
    });

    candleSeries = chart.addSeries(CandlestickSeries, {
      upColor: "#70f7b1",
      downColor: "#ff7f7f",
      wickUpColor: "#70f7b1",
      wickDownColor: "#ff7f7f",
      borderVisible: false,
    });

    volumeSeries = chart.addSeries(HistogramSeries, {
      priceFormat: {
        type: "volume",
      },
      priceScaleId: "",
    });

    volumeSeries.priceScale().applyOptions({
      scaleMargins: {
        top: 0.78,
        bottom: 0,
      },
    });

    candleSeries.priceScale().applyOptions({
      scaleMargins: {
        top: 0.08,
        bottom: 0.28,
      },
    });

    if (showVwap) {
      import("lightweight-charts").then(({ LineSeries }) => {
        vwapSeries = chart?.addSeries(LineSeries, {
          color: "#f0b450",
          lineWidth: 2,
          crosshairMarkerVisible: false,
          priceLineVisible: false,
        });
        syncSeries();
      });
    }

    syncSeries();
    resize();

    const observer = new ResizeObserver(resize);
    observer.observe(container);

    return () => {
      observer.disconnect();
      chart?.remove();
      chart = undefined;
    };
  });

  $: syncSeries();
</script>

<section class="chart-shell">
  <div class="chart-header">
    <div>
      <p>Interactive Ticker</p>
      <h2>{symbol}</h2>
    </div>
    <span>{candles.length} candles loaded</span>
  </div>

  <div bind:this={container} class="chart-frame" style="min-height: {height}px;"></div>
</section>

<style>
  .chart-shell {
    padding: 1.25rem;
    border-radius: 26px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background:
      radial-gradient(circle at top left, rgba(67, 94, 190, 0.22), transparent 34%),
      linear-gradient(180deg, rgba(17, 22, 37, 0.96), rgba(11, 14, 24, 0.92));
  }

  .chart-header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: baseline;
    margin-bottom: 1rem;
  }

  .chart-header p,
  .chart-header span {
    margin: 0;
    color: rgba(221, 233, 255, 0.65);
    font-size: 0.92rem;
  }

  .chart-header h2 {
    margin: 0.2rem 0 0;
    font-size: 1.8rem;
    color: white;
  }

  .chart-frame {
    width: 100%;
  }
</style>
