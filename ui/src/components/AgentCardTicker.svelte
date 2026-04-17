<script lang="ts">
  import { onMount } from "svelte";
  import {
    CandlestickSeries,
    ColorType,
    createChart,
    type CandlestickData,
    type IChartApi,
    type ISeriesApi,
    type Time,
    type UTCTimestamp,
  } from "lightweight-charts";
  import { api, apiTokenConfigured } from "../lib/api";
  import type { Candle } from "../lib/types";

  export let symbol: string;
  export let showVwap: boolean = false;
  export let height: number = 120;

  let container: HTMLDivElement;
  let chart: IChartApi | undefined;
  let candleSeries: ISeriesApi<"Candlestick"> | undefined;
  let vwapSeries: ISeriesApi<"Line"> | undefined;
  let candles: Candle[] = [];

  function toSeriesData(series: Candle[]): CandlestickData<Time>[] {
    return series.map((candle) => ({
      time: (Date.parse(candle.timestamp) / 1000) as UTCTimestamp,
      open: candle.open,
      high: candle.high,
      low: candle.low,
      close: candle.close,
    }));
  }

  function syncSeries() {
    if (!candleSeries || candles.length === 0) return;
    candleSeries.setData(toSeriesData(candles));

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
    void (async () => {
      if (apiTokenConfigured) {
        try {
          const resp = await fetch(
            `${import.meta.env.VITE_API_BASE_URL ?? "http://127.0.0.1:8080"}/api/market/candles/${symbol}?range=1d&interval=5m`,
            {
              headers: {
                Authorization: `Bearer ${import.meta.env.VITE_API_TOKEN ?? ""}`,
              },
            },
          );
          if (resp.ok) {
            candles = await resp.json();
          }
        } catch {
          // Silently fail – the card just won't show a chart.
        }
      }

      syncSeries();
      resize();
    })();

    chart = createChart(container, {
      width: container.clientWidth,
      height,
      layout: {
        background: { type: ColorType.Solid, color: "transparent" },
        textColor: "#8fa4c4",
        fontSize: 10,
      },
      grid: {
        vertLines: { visible: false },
        horzLines: { visible: false },
      },
      rightPriceScale: { borderVisible: false },
      timeScale: { visible: false },
      crosshair: {
        vertLine: { visible: false },
        horzLine: { visible: false },
      },
    });

    candleSeries = chart.addSeries(CandlestickSeries, {
      upColor: "#70f7b1",
      downColor: "#ff7f7f",
      wickUpColor: "#70f7b1",
      wickDownColor: "#ff7f7f",
      borderVisible: false,
    });

    if (showVwap) {
      import("lightweight-charts").then(({ LineSeries }) => {
        vwapSeries = chart?.addSeries(LineSeries, {
          color: "#f0b450",
          lineWidth: 1,
          crosshairMarkerVisible: false,
          priceLineVisible: false,
        });
        syncSeries();
      });
    }

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

<div bind:this={container} class="agent-card-chart" style="min-height: {height}px;"></div>

<style>
  .agent-card-chart {
    border-radius: 12px;
    overflow: hidden;
  }
</style>
