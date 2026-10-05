<script lang="ts">
  import { onMount } from 'svelte';
  import type { EqConfig, EqBand, FilterType } from '../types';

  export let eqConfig: EqConfig;
  export let accentColor: string = '#22c55e';
  export let onChange: (config: EqConfig) => void;

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null = null;
  let selectedBandIndex: number | null = null;
  let isDragging = false;

  const MIN_FREQ = 20;
  const MAX_FREQ = 20000;
  const MIN_GAIN = -12;
  const MAX_GAIN = 12;

  function freqToX(freq: number, width: number): number {
    const logMin = Math.log10(MIN_FREQ);
    const logMax = Math.log10(MAX_FREQ);
    return ((Math.log10(freq) - logMin) / (logMax - logMin)) * width;
  }

  function xToFreq(x: number, width: number): number {
    const logMin = Math.log10(MIN_FREQ);
    const logMax = Math.log10(MAX_FREQ);
    const clampedX = Math.max(0, Math.min(width, x));
    return Math.pow(10, logMin + (clampedX / width) * (logMax - logMin));
  }

  function gainToY(gain: number, height: number): number {
    const clamped = Math.max(MIN_GAIN, Math.min(MAX_GAIN, gain));
    return height / 2 - (clamped / MAX_GAIN) * (height / 2 * 0.85);
  }

  function yToGain(y: number, height: number): number {
    const norm = (height / 2 - y) / (height / 2 * 0.85);
    return Math.max(MIN_GAIN, Math.min(MAX_GAIN, norm * MAX_GAIN));
  }

  // Calculates combined frequency response across all bands at frequency f
  function calculateTotalGain(f: number): number {
    let total = 0;
    for (const band of eqConfig.bands) {
      if (!band.enabled) continue;
      const f0 = band.freq_hz;
      const g = band.gain_db;
      const q = band.q;

      if (band.filter_type === 'low_shelf') {
        const factor = 1 / (1 + Math.pow(f / f0, 2));
        total += g * factor;
      } else if (band.filter_type === 'high_shelf') {
        const factor = 1 / (1 + Math.pow(f0 / f, 2));
        total += g * factor;
      } else {
        // Peaking Bell filter approximation
        const octDiff = Math.abs(Math.log2(f / f0));
        const bandwidth = 1.0 / q;
        const bell = Math.exp(-Math.pow(octDiff / (bandwidth * 0.7), 2));
        total += g * bell;
      }
    }

    // Apply quick tone adjustments
    if (f <= 250) {
      total += eqConfig.bass_db * (1 - f / 300);
    } else if (f >= 500 && f <= 2500) {
      const midDist = Math.abs(f - 1200) / 1000;
      total += eqConfig.voice_db * Math.max(0, 1 - midDist);
    } else if (f >= 4000) {
      total += eqConfig.treble_db * Math.min(1, (f - 3000) / 5000);
    }

    return total;
  }

  export function draw() {
    if (!canvas) return;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    const dpr = window.devicePixelRatio || 1;

    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    // 1. Clear background
    ctx.fillStyle = '#14181f';
    ctx.fillRect(0, 0, width, height);

    // 2. Frequency range regions
    const regions = [
      { name: 'SUB BASS', start: 20, end: 60, col: 'rgba(255, 255, 255, 0.015)' },
      { name: 'BASS', start: 60, end: 250, col: 'rgba(255, 255, 255, 0.025)' },
      { name: 'LOW MIDS', start: 250, end: 500, col: 'rgba(255, 255, 255, 0.015)' },
      { name: 'MID RANGE', start: 500, end: 2000, col: 'rgba(255, 255, 255, 0.025)' },
      { name: 'UPPER MIDS', start: 2000, end: 6000, col: 'rgba(255, 255, 255, 0.015)' },
      { name: 'HIGHS', start: 6000, end: 20000, col: 'rgba(255, 255, 255, 0.025)' },
    ];

    ctx.font = '10px sans-serif';
    ctx.textAlign = 'center';
    for (const r of regions) {
      const x1 = freqToX(r.start, width);
      const x2 = freqToX(r.end, width);
      ctx.fillStyle = r.col;
      ctx.fillRect(x1, 0, x2 - x1, height);

      ctx.fillStyle = '#4b5563';
      ctx.fillText(r.name, (x1 + x2) / 2, 16);
    }

    // 3. Grid lines (dB lines)
    ctx.lineWidth = 1;
    ctx.strokeStyle = '#222933';
    ctx.fillStyle = '#64748b';
    ctx.font = '10px monospace';
    ctx.textAlign = 'right';

    for (const db of [-12, -6, 0, 6, 12]) {
      const y = gainToY(db, height);
      ctx.beginPath();
      ctx.strokeStyle = db === 0 ? '#334155' : '#1e2632';
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();

      ctx.fillText(`${db > 0 ? '+' : ''}${db} dB`, width - 8, y - 3);
    }

    // Frequency grid vertical lines
    const freqs = [50, 100, 250, 500, 1000, 2000, 4000, 8000, 16000];
    ctx.textAlign = 'center';
    for (const f of freqs) {
      const x = freqToX(f, width);
      ctx.beginPath();
      ctx.strokeStyle = '#1e2632';
      ctx.moveTo(x, 24);
      ctx.lineTo(x, height);
      ctx.stroke();

      const label = f >= 1000 ? `${f / 1000}k` : `${f}`;
      ctx.fillText(label, x, height - 6);
    }

    // 4. Draw EQ response curve
    const step = 2;
    ctx.beginPath();
    let first = true;
    for (let x = 0; x <= width; x += step) {
      const f = xToFreq(x, width);
      const gain = calculateTotalGain(f);
      const y = gainToY(gain, height);
      if (first) {
        ctx.moveTo(x, y);
        first = false;
      } else {
        ctx.lineTo(x, y);
      }
    }

    // Fill under curve
    ctx.strokeStyle = accentColor;
    ctx.lineWidth = 2.5;
    ctx.stroke();

    ctx.lineTo(width, height / 2);
    ctx.lineTo(0, height / 2);
    ctx.closePath();
    const grad = ctx.createLinearGradient(0, 0, 0, height);
    grad.addColorStop(0, `${accentColor}33`);
    grad.addColorStop(1, `${accentColor}05`);
    ctx.fillStyle = grad;
    ctx.fill();

    // 5. Draw 10 draggable band nodes
    eqConfig.bands.forEach((band, index) => {
      if (!band.enabled) return;
      const x = freqToX(band.freq_hz, width);
      const y = gainToY(band.gain_db, height);
      const isSelected = selectedBandIndex === index;

      // Glow ring if selected
      if (isSelected) {
        ctx!.beginPath();
        ctx!.arc(x, y, 12, 0, Math.PI * 2);
        ctx!.fillStyle = `${accentColor}44`;
        ctx!.fill();
      }

      ctx!.beginPath();
      ctx!.arc(x, y, 7, 0, Math.PI * 2);
      ctx!.fillStyle = isSelected ? '#ffffff' : accentColor;
      ctx!.fill();
      ctx!.strokeStyle = '#0f172a';
      ctx!.lineWidth = 2;
      ctx!.stroke();

      // Band number inside/beside
      ctx!.fillStyle = '#ffffff';
      ctx!.font = '9px sans-serif';
      ctx!.textAlign = 'center';
      ctx!.fillText(`${index + 1}`, x, y + 3);
    });
  }

  function handleMouseDown(e: MouseEvent) {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    // Hit test bands
    let found = -1;
    for (let i = 0; i < eqConfig.bands.length; i++) {
      const bx = freqToX(eqConfig.bands[i].freq_hz, canvas.clientWidth);
      const by = gainToY(eqConfig.bands[i].gain_db, canvas.clientHeight);
      const dist = Math.hypot(x - bx, y - by);
      if (dist <= 14) {
        found = i;
        break;
      }
    }

    if (found !== -1) {
      selectedBandIndex = found;
      isDragging = true;
      draw();
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isDragging || selectedBandIndex === null || !canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const newFreq = Math.round(xToFreq(x, canvas.clientWidth));
    const newGain = Math.round(yToGain(y, canvas.clientHeight) * 10) / 10;

    eqConfig.bands[selectedBandIndex].freq_hz = Math.max(MIN_FREQ, Math.min(MAX_FREQ, newFreq));
    eqConfig.bands[selectedBandIndex].gain_db = Math.max(MIN_GAIN, Math.min(MAX_GAIN, newGain));

    draw();
    onChange(eqConfig);
  }

  function handleMouseUp() {
    isDragging = false;
  }

  function handleDoubleClick(e: MouseEvent) {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    for (let i = 0; i < eqConfig.bands.length; i++) {
      const bx = freqToX(eqConfig.bands[i].freq_hz, canvas.clientWidth);
      const by = gainToY(eqConfig.bands[i].gain_db, canvas.clientHeight);
      if (Math.hypot(x - bx, y - by) <= 14) {
        eqConfig.bands[i].gain_db = 0.0;
        draw();
        onChange(eqConfig);
        break;
      }
    }
  }

  onMount(() => {
    draw();
    window.addEventListener('resize', draw);
    return () => window.removeEventListener('resize', draw);
  });

  $: if (eqConfig || accentColor) {
    draw();
  }
</script>

<div class="eq-graph-container">
  <canvas
    bind:this={canvas}
    on:mousedown={handleMouseDown}
    on:mousemove={handleMouseMove}
    on:mouseup={handleMouseUp}
    on:mouseleave={handleMouseUp}
    on:dblclick={handleDoubleClick}
  ></canvas>

  {#if selectedBandIndex !== null}
    {@const b = eqConfig.bands[selectedBandIndex]}
    <div class="inspector-badge">
      <span class="badge-title">BAND {selectedBandIndex + 1}</span>
      <span class="badge-val">{b.freq_hz >= 1000 ? (b.freq_hz / 1000).toFixed(1) + ' kHz' : b.freq_hz + ' Hz'}</span>
      <span class="badge-val">{b.gain_db > 0 ? '+' : ''}{b.gain_db.toFixed(1)} dB</span>
      <span class="badge-val">Q: {b.q.toFixed(2)}</span>
    </div>
  {/if}
</div>

<style>
  .eq-graph-container {
    position: relative;
    width: 100%;
    height: 240px;
    background: #14181f;
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid #242c38;
  }

  canvas {
    width: 100%;
    height: 100%;
    display: block;
    cursor: crosshair;
  }

  .inspector-badge {
    position: absolute;
    bottom: 10px;
    left: 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(15, 23, 42, 0.85);
    backdrop-filter: blur(6px);
    border: 1px solid #334155;
    padding: 3px 8px;
    border-radius: 6px;
    font-size: 11px;
    user-select: none;
  }

  .badge-title {
    font-weight: 700;
    color: #94a3b8;
  }

  .badge-val {
    font-family: monospace;
    color: #f1f5f9;
  }
</style>
