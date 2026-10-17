<script lang="ts">
  import { onMount } from 'svelte';
  import type { EqConfig, EqBand, FilterType, ChannelMeter } from '../types';
  import CustomSelect, { type SelectOption } from './CustomSelect.svelte';

  export let eqConfig: EqConfig;
  export let meter: ChannelMeter | undefined = undefined;
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

  // Computes exact magnitude response in dB for an RBJ biquad filter at frequency f
  function computeBiquadGainDb(band: EqBand, f: number, fs: number = 48000): number {
    if (!band.enabled || (band.gain_db === 0 && band.filter_type !== 'low_pass' && band.filter_type !== 'high_pass')) {
      return 0;
    }
    const f0 = Math.max(10, Math.min(fs * 0.49, band.freq_hz));
    const gainDb = band.gain_db;
    const q = Math.max(0.1, band.q);

    const w0 = (2 * Math.PI * f0) / fs;
    const cosW0 = Math.cos(w0);
    const sinW0 = Math.sin(w0);
    const alpha = sinW0 / (2 * q);
    const A = Math.pow(10, gainDb / 40);

    let b0 = 1, b1 = 0, b2 = 0, a0 = 1, a1 = 0, a2 = 0;

    switch (band.filter_type) {
      case 'peak': {
        b0 = 1 + alpha * A;
        b1 = -2 * cosW0;
        b2 = 1 - alpha * A;
        a0 = 1 + alpha / A;
        a1 = -2 * cosW0;
        a2 = 1 - alpha / A;
        break;
      }
      case 'low_shelf': {
        const twoSqrtAAlpha = 2 * Math.sqrt(A) * alpha;
        b0 = A * ((A + 1) - (A - 1) * cosW0 + twoSqrtAAlpha);
        b1 = 2 * A * ((A - 1) - (A + 1) * cosW0);
        b2 = A * ((A + 1) - (A - 1) * cosW0 - twoSqrtAAlpha);
        a0 = (A + 1) + (A - 1) * cosW0 + twoSqrtAAlpha;
        a1 = -2 * ((A - 1) + (A + 1) * cosW0);
        a2 = (A + 1) + (A - 1) * cosW0 - twoSqrtAAlpha;
        break;
      }
      case 'high_shelf': {
        const twoSqrtAAlpha = 2 * Math.sqrt(A) * alpha;
        b0 = A * ((A + 1) + (A - 1) * cosW0 + twoSqrtAAlpha);
        b1 = -2 * A * ((A - 1) + (A + 1) * cosW0);
        b2 = A * ((A + 1) + (A - 1) * cosW0 - twoSqrtAAlpha);
        a0 = (A + 1) - (A - 1) * cosW0 + twoSqrtAAlpha;
        a1 = 2 * ((A - 1) - (A + 1) * cosW0);
        a2 = (A + 1) - (A - 1) * cosW0 - twoSqrtAAlpha;
        break;
      }
      case 'high_pass': {
        b0 = (1 + cosW0) / 2;
        b1 = -(1 + cosW0);
        b2 = (1 + cosW0) / 2;
        a0 = 1 + alpha;
        a1 = -2 * cosW0;
        a2 = 1 - alpha;
        break;
      }
      case 'low_pass': {
        b0 = (1 - cosW0) / 2;
        b1 = 1 - cosW0;
        b2 = (1 - cosW0) / 2;
        a0 = 1 + alpha;
        a1 = -2 * cosW0;
        a2 = 1 - alpha;
        break;
      }
      default:
        return 0;
    }

    b0 /= a0;
    b1 /= a0;
    b2 /= a0;
    a1 /= a0;
    a2 /= a0;

    const w = (2 * Math.PI * f) / fs;
    const cosW = Math.cos(w);
    const cos2W = Math.cos(2 * w);
    const sinW = Math.sin(w);
    const sin2W = Math.sin(2 * w);

    const numReal = b0 + b1 * cosW + b2 * cos2W;
    const numImag = -b1 * sinW - b2 * sin2W;
    const denReal = 1 + a1 * cosW + a2 * cos2W;
    const denImag = -a1 * sinW - a2 * sin2W;

    const numMagSq = numReal * numReal + numImag * numImag;
    const denMagSq = denReal * denReal + denImag * denImag;

    if (denMagSq < 1e-12) return 0;
    return 10 * Math.log10(Math.max(1e-12, numMagSq / denMagSq));
  }

  // Calculates combined frequency response across all bands and macro tones at frequency f
  function calculateTotalGain(f: number): number {
    let total = 0;
    for (const band of eqConfig.bands) {
      total += computeBiquadGainDb(band, f);
    }

    // Macro tone shelf and peak biquads (matching voicegg-dsp ParametricEq)
    if (eqConfig.bass_db) {
      total += computeBiquadGainDb(
        { freq_hz: 120, gain_db: eqConfig.bass_db, q: 0.707, filter_type: 'low_shelf', enabled: true },
        f
      );
    }
    if (eqConfig.voice_db) {
      total += computeBiquadGainDb(
        { freq_hz: 1200, gain_db: eqConfig.voice_db, q: 1.0, filter_type: 'peak', enabled: true },
        f
      );
    }
    if (eqConfig.treble_db) {
      total += computeBiquadGainDb(
        { freq_hz: 6000, gain_db: eqConfig.treble_db, q: 0.707, filter_type: 'high_shelf', enabled: true },
        f
      );
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
    ctx.fillStyle = '#11151F';
    ctx.fillRect(0, 0, width, height);

    // 2. Frequency range regions
    const regions = [
      { name: 'SUB BASS', start: 20, end: 60, col: 'rgba(255, 255, 255, 0.012)' },
      { name: 'BASS', start: 60, end: 250, col: 'rgba(255, 255, 255, 0.022)' },
      { name: 'LOW MIDS', start: 250, end: 500, col: 'rgba(255, 255, 255, 0.012)' },
      { name: 'MID RANGE', start: 500, end: 2000, col: 'rgba(255, 255, 255, 0.022)' },
      { name: 'UPPER MIDS', start: 2000, end: 6000, col: 'rgba(255, 255, 255, 0.012)' },
      { name: 'HIGHS', start: 6000, end: 20000, col: 'rgba(255, 255, 255, 0.022)' },
    ];

    ctx.font = '10px "IBM Plex Sans Arabic", "Inter", sans-serif';
    ctx.textAlign = 'center';
    for (const r of regions) {
      const x1 = freqToX(r.start, width);
      const x2 = freqToX(r.end, width);
      ctx.fillStyle = r.col;
      ctx.fillRect(x1, 0, x2 - x1, height);

      ctx.fillStyle = '#64748B';
      ctx.fillText(r.name, (x1 + x2) / 2, 16);
    }

    // 3. Grid lines (dB lines)
    ctx.lineWidth = 1;
    ctx.strokeStyle = '#1F2637';
    ctx.fillStyle = '#64748B';
    ctx.font = '10px "JetBrains Mono", monospace';
    ctx.textAlign = 'right';

    for (const db of [-12, -6, 0, 6, 12]) {
      const y = gainToY(db, height);
      ctx.beginPath();
      ctx.strokeStyle = db === 0 ? '#2A344A' : '#1F2637';
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();

      ctx.fillText(`${db > 0 ? '+' : ''}${db} dB`, width - 8, y - 3);
    }

    // Frequency grid vertical lines
    const freqs = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
    ctx.textAlign = 'center';
    ctx.font = '10px "JetBrains Mono", monospace';
    for (const f of freqs) {
      const x = freqToX(f, width);
      ctx.beginPath();
      ctx.strokeStyle = '#1F2637';
      ctx.moveTo(x, 24);
      ctx.lineTo(x, height);
      ctx.stroke();

      const label = f >= 1000 ? `${f / 1000}k` : `${f}`;
      ctx.fillText(label, x, height - 6);
    }

    // 3.5. Live RTA Spectrum Analyzer Backdrop
    if (meter && meter.rms_dbfs > -60) {
      const numBars = 48;
      const barWidth = width / numBars;
      const signalNorm = Math.max(0, Math.min(1, (meter.rms_dbfs + 60) / 60));
      for (let i = 0; i < numBars; i++) {
        const barX = i * barWidth;
        const centerWeight = Math.sin((i / numBars) * Math.PI);
        const barHeight = Math.max(3, signalNorm * (height * 0.5) * (0.35 + 0.65 * centerWeight));
        const barY = height - barHeight;

        ctx.fillStyle = `${accentColor}18`;
        ctx.fillRect(barX + 1, barY, barWidth - 2, barHeight);

        ctx.fillStyle = `${accentColor}40`;
        ctx.fillRect(barX + 1, barY, barWidth - 2, 2);
      }
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
        selectedBandIndex = i;
        draw();
        onChange(eqConfig);
        break;
      }
    }
  }

  function handleWheel(e: WheelEvent) {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    // Check if mouse is over any band node or currently selected band
    let targetIndex = -1;
    for (let i = 0; i < eqConfig.bands.length; i++) {
      const bx = freqToX(eqConfig.bands[i].freq_hz, canvas.clientWidth);
      const by = gainToY(eqConfig.bands[i].gain_db, canvas.clientHeight);
      if (Math.hypot(x - bx, y - by) <= 16) {
        targetIndex = i;
        break;
      }
    }

    if (targetIndex === -1 && selectedBandIndex !== null) {
      const bx = freqToX(eqConfig.bands[selectedBandIndex].freq_hz, canvas.clientWidth);
      const by = gainToY(eqConfig.bands[selectedBandIndex].gain_db, canvas.clientHeight);
      if (Math.hypot(x - bx, y - by) <= 24) {
        targetIndex = selectedBandIndex;
      }
    }

    if (targetIndex !== -1) {
      e.preventDefault();
      selectedBandIndex = targetIndex;
      // Wheel up (e.deltaY < 0) increases Q (narrower bandwidth), wheel down decreases Q (0.2 to 10.0)
      const delta = e.deltaY < 0 ? 0.2 : -0.2;
      const currentQ = eqConfig.bands[targetIndex].q ?? 1.0;
      const newQ = Math.round(Math.max(0.2, Math.min(10.0, currentQ + delta)) * 10) / 10;
      eqConfig.bands[targetIndex].q = newQ;
      draw();
      onChange(eqConfig);
    }
  }

  const filterTypeOptions: SelectOption[] = [
    { value: 'peak', label: 'Peak' },
    { value: 'low_shelf', label: 'Low Shelf' },
    { value: 'high_shelf', label: 'High Shelf' },
    { value: 'high_pass', label: 'High Pass' },
    { value: 'low_pass', label: 'Low Pass' },
  ];

  function handleFilterTypeChange(index: number, type: FilterType) {
    if (index >= 0 && index < eqConfig.bands.length) {
      eqConfig.bands[index].filter_type = type;
      draw();
      onChange(eqConfig);
    }
  }

  function toggleBandEnabled(index: number) {
    if (index >= 0 && index < eqConfig.bands.length) {
      eqConfig.bands[index].enabled = !eqConfig.bands[index].enabled;
      draw();
      onChange(eqConfig);
    }
  }

  onMount(() => {
    draw();
    window.addEventListener('resize', draw);
    return () => window.removeEventListener('resize', draw);
  });

  $: if (eqConfig || accentColor || meter) {
    draw();
  }
</script>

<div class="eq-component-wrapper">
  <div class="eq-graph-container">
    <canvas
      bind:this={canvas}
      on:mousedown={handleMouseDown}
      on:mousemove={handleMouseMove}
      on:mouseup={handleMouseUp}
      on:mouseleave={handleMouseUp}
      on:dblclick={handleDoubleClick}
      on:wheel={handleWheel}
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

  <!-- 10-Band Parameter Inspector Table -->
  <div class="eq-table-card">
    <div class="table-header-title">
      <span class="table-title">PARAMETRIC EQUALIZER BANDS</span>
      <span class="table-hint">Double-click node to reset • Mouse wheel adjusts Q</span>
    </div>

    <div class="table-scroll-container">
      <table class="eq-bands-table">
        <thead>
          <tr>
            <th class="th-num">#</th>
            <th class="th-type">TYPE</th>
            <th class="th-freq">FREQ</th>
            <th class="th-gain">GAIN</th>
            <th class="th-q">Q</th>
            <th class="th-bypass">STATE</th>
          </tr>
        </thead>
        <tbody>
          {#each eqConfig.bands as band, i}
            <tr
              class="band-row"
              class:is-selected={selectedBandIndex === i}
              class:is-bypassed={!band.enabled}
              on:click={() => { selectedBandIndex = i; draw(); }}
            >
              <td class="col-num">
                <span
                  class="num-dot"
                  style="background-color: {band.enabled ? accentColor : 'var(--color-neutral-500, #64748B)'};"
                ></span>
                <span class="num-text">{i + 1}</span>
              </td>
              <td class="col-type" on:click|stopPropagation>
                <CustomSelect
                  size="sm"
                  value={band.filter_type}
                  options={filterTypeOptions}
                  onChange={(val) => handleFilterTypeChange(i, val as FilterType)}
                />
              </td>
              <td class="col-freq" on:click|stopPropagation>
                <div class="num-input-wrap">
                  <input
                    type="number"
                    min="20"
                    max="20000"
                    step="1"
                    class="table-num-input"
                    bind:value={band.freq_hz}
                    on:input={() => { draw(); onChange(eqConfig); }}
                  />
                  <span class="unit">Hz</span>
                </div>
              </td>
              <td class="col-gain" on:click|stopPropagation>
                <div class="num-input-wrap">
                  <input
                    type="number"
                    min="-12"
                    max="12"
                    step="0.5"
                    class="table-num-input"
                    bind:value={band.gain_db}
                    on:input={() => { draw(); onChange(eqConfig); }}
                  />
                  <span class="unit">dB</span>
                </div>
              </td>
              <td class="col-q" on:click|stopPropagation>
                <div class="num-input-wrap">
                  <input
                    type="number"
                    min="0.2"
                    max="10"
                    step="0.1"
                    class="table-num-input"
                    bind:value={band.q}
                    on:input={() => { draw(); onChange(eqConfig); }}
                  />
                </div>
              </td>
              <td class="col-bypass" on:click|stopPropagation>
                <button
                  type="button"
                  class="band-toggle-btn"
                  class:active={band.enabled}
                  on:click={() => toggleBandEnabled(i)}
                  title={band.enabled ? 'Bypass Band' : 'Enable Band'}
                >
                  {band.enabled ? 'ACTIVE' : 'BYPASS'}
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</div>

<style>
  .eq-component-wrapper {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
    font-family: var(--font-sans);
  }

  .eq-graph-container {
    position: relative;
    width: 100%;
    height: 220px;
    background: var(--color-surface-1, #11151F);
    border-radius: var(--radius-md, 6px);
    overflow: hidden;
    border: 1px solid var(--color-border-default, #1F2637);
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
    background: rgba(17, 21, 31, 0.9);
    backdrop-filter: blur(6px);
    border: 1px solid var(--color-border-bright, #2A344A);
    padding: 4px 10px;
    border-radius: var(--radius-sm, 4px);
    font-size: 11px;
    user-select: none;
    box-shadow: var(--shadow-depth-sm, 0 1px 2px rgba(0,0,0,0.3));
  }

  .badge-title {
    font-weight: 700;
    color: var(--color-text-secondary, #94A3B8);
  }

  .badge-val {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    color: var(--color-text-primary, #F8FAFC);
  }

  /* Table Card */
  .eq-table-card {
    background: var(--color-surface-1, #11151F);
    border: 1px solid var(--color-border-default, #1F2637);
    border-radius: var(--radius-md, 6px);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .table-header-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .table-title {
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-neutral-300, #CBD5E1);
  }

  .table-hint {
    font-size: 0.7rem;
    color: var(--color-neutral-500, #64748B);
  }

  .table-scroll-container {
    overflow-x: auto;
    max-height: 220px;
    overflow-y: auto;
  }

  .eq-bands-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.775rem;
  }

  .eq-bands-table th {
    text-align: left;
    padding: 6px 8px;
    font-size: 0.65rem;
    font-weight: 700;
    color: var(--color-neutral-500, #64748B);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border-bottom: 1px solid var(--color-border-default, #1F2637);
  }

  .eq-bands-table td {
    padding: 5px 8px;
    border-bottom: 1px solid var(--color-border-subtle, #1C2230);
    vertical-align: middle;
  }

  .band-row {
    cursor: pointer;
    transition: background var(--transition-fast, 120ms);
  }

  .band-row:hover {
    background: var(--color-surface-hover, #252D40);
  }

  .band-row.is-selected {
    background: rgba(255, 82, 0, 0.08);
  }

  .band-row.is-bypassed {
    opacity: 0.45;
  }

  .col-num {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 700;
    font-family: var(--font-mono);
  }

  .num-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .col-type {
    width: 130px;
  }

  .num-input-wrap {
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--color-surface-2, #161B26);
    border: 1px solid var(--color-border-default, #1F2637);
    border-radius: var(--radius-sm, 4px);
    padding: 3px 6px;
    width: 75px;
  }

  .table-num-input {
    width: 100%;
    background: transparent;
    border: none;
    color: var(--color-text-primary, #F8FAFC);
    font-family: var(--font-mono);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
    outline: none;
    padding: 0;
  }

  .unit {
    font-size: 0.65rem;
    color: var(--color-neutral-500, #64748B);
    font-family: var(--font-mono);
  }

  .band-toggle-btn {
    padding: 3px 8px;
    border-radius: var(--radius-sm, 4px);
    font-size: 0.65rem;
    font-weight: 700;
    font-family: var(--font-mono);
    border: 1px solid var(--color-border-bright, #2A344A);
    background: var(--color-surface-2, #161B26);
    color: var(--color-neutral-400, #94A3B8);
    cursor: pointer;
    transition: all var(--transition-fast, 120ms);
  }

  .band-toggle-btn.active {
    background: rgba(16, 185, 129, 0.15);
    color: var(--color-accent-success, #10B981);
    border-color: rgba(16, 185, 129, 0.35);
  }

  .band-toggle-btn:hover {
    border-color: var(--color-neutral-400, #94A3B8);
  }
</style>
