<script lang="ts">
  import { onMount } from 'svelte';

  export let data: number[] = [];
  export let color: string = '#22c55e';
  export let width: number = 100;
  export let height: number = 30;

  let canvas: HTMLCanvasElement;

  function draw() {
    if (!canvas || data.length < 2) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    ctx.clearRect(0, 0, width, height);
    
    // Scale points
    const min = Math.min(...data, 0);
    const max = Math.max(...data, 1);
    const range = max - min;

    const points = data.map((val, i) => ({
      x: (i / (data.length - 1)) * width,
      y: height - ((val - min) / range) * height
    }));

    // Draw line
    ctx.beginPath();
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.lineJoin = 'round';
    ctx.lineCap = 'round';

    ctx.moveTo(points[0].x, points[0].y);
    for (let i = 1; i < points.length; i++) {
      ctx.lineTo(points[i].x, points[i].y);
    }
    ctx.stroke();

    // Draw area gradient
    const gradient = ctx.createLinearGradient(0, 0, 0, height);
    gradient.addColorStop(0, `${color}33`);
    gradient.addColorStop(1, `${color}00`);
    
    ctx.lineTo(points[points.length - 1].x, height);
    ctx.lineTo(points[0].x, height);
    ctx.fillStyle = gradient;
    ctx.fill();
  }

  onMount(() => {
    draw();
  });

  $: if (data) draw();
</script>

<canvas 
  bind:this={canvas} 
  {width} 
  {height} 
  style="display: block; width: {width}px; height: {height}px;"
></canvas>
