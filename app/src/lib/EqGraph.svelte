<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { EqConfig, EqBand, FilterType, ChannelMeter } from '../types';
  import CustomSelect, { type SelectOption } from './CustomSelect.svelte';

  export let eqConfig: EqConfig;
  export let meter: ChannelMeter | undefined = undefined;
  export let accentColor: string = '#00E599';
  export let onChange: (config: EqConfig) => void;

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null = null;
  let selectedBandIndex: number = 0;
  let isDragging = false;

  // Local working copy for zero-snap 60fps pointer dragging
  let localEq: EqConfig = cloneEq(eqConfig);
  let lastSyncedJson = JSON.stringify(eqConfig);
  let throttleTimer: ReturnType<typeof setTimeout> | null = null;
  let lastEmitAt = 0;

  function cloneEq(cfg: EqConfig): EqConfig {
    return {
      enabled: cfg?.enabled ?? true,
      bass_db: cfg?.bass_db ?? 0,
      voice_db: cfg?.voice_db ?? 0,
      treble_db: cfg?.treble_db ?? 0,
      bands: (cfg?.bands || []).map((b) => ({ ...b })),
    };
  }

  $: {
    const incomingJson = JSON.stringify(eqConfig);
    if (!isDragging && incomingJson !== lastSyncedJson) {
      localEq = cloneEq(eqConfig);
      lastSyncedJson = incomingJson;
      if (selectedBandIndex >= localEq.bands.length) {
        selectedBandIndex = 0;
      }
      draw();
    }
  }

  function emitChange(immediate = false) {
    lastSyncedJson = JSON.stringify(localEq);
    const now = Date.now();
    if (immediate || now - lastEmitAt >= 40) {
      if (throttleTimer) {
        clearTimeout(throttleTimer);
        throttleTimer = null;
      }
      lastEmitAt = now;
      onChange(cloneEq(localEq));
    } else if (!throttleTimer) {
      const remaining = 40 - (now - lastEmitAt);
      throttleTimer = setTimeout(() => {
        throttleTimer = null;
        lastEmitAt = Date.now();
        onChange(cloneEq(localEq));
      }, remaining);
    }
  }

  const MIN_FREQ = 20;
  const MAX_FREQ = 20000;
  const MIN_GAIN = -12;
  const MAX_GAIN = 12;

  function freqToX(freq: number, width: number): number {
    const logMin = Math.log10(MIN_FREQ);
    const logMax = Math.log10(MAX_FREQ);
    return ((Math.log10(Math.max(MIN_FREQ, Math.min(MAX_FREQ, freq))) - logMin) / (logMax - logMin)) * width;
  }

  function xToFreq(x: number, width: number): number {
    const logMin = Math.log10(MIN_FREQ);
    const logMax = Math.log10(MAX_FREQ);
    const clampedX = Math.max(0, Math.min(width, x));
    return Math.pow(10, logMin + (clampedX / width) * (logMax - logMin));
  }

  function gainToY(gain: number, height: number): number {
    const clamped = Math.max(MIN_GAIN, Math.min(MAX_GAIN, gain));
    return height / 2 - (clamped / MAX_GAIN) * ((height / 2) * 0.82);
  }

  function yToGain(y: number, height: number): number {
    const norm = (height / 2 - y) / ((height / 2) * 0.82);
    return Math.max(MIN_GAIN, Math.min(MAX_GAIN, norm * MAX_GAIN));
  }

  // Computes exact magnitude response in dB for an RBJ biquad filter at frequency f
  function computeBiquadGainDb(band: EqBand, f: number, fs: number = 48000): number {
    if (!band.enabled) return 0;
    if (band.gain_db === 0 && band.filter_type !== 'low_pass' && band.filter_type !== 'high_pass' && band.filter_type !== 'notch') {
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
      case 'notch': {
        b0 = 1;
        b1 = -2 * cosW0;
        b2 = 1;
        a0 = 1 + alpha;
        a1 = -2 * cosW0;
        a2 = 1 - alpha;
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

  function getMacroGainAt(f: number): number {
    let macro = 0;
    if (localEq.bass_db) {
      macro += computeBiquadGainDb(
        { freq_hz: 120, gain_db: localEq.bass_db, q: 0.707, filter_type: 'low_shelf', enabled: true },
        f
      );
    }
    if (localEq.voice_db) {
      macro += computeBiquadGainDb(
        { freq_hz: 1200, gain_db: localEq.voice_db, q: 1.0, filter_type: 'peak', enabled: true },
        f
      );
    }
    if (localEq.treble_db) {
      macro += computeBiquadGainDb(
        { freq_hz: 6000, gain_db: localEq.treble_db, q: 0.707, filter_type: 'high_shelf', enabled: true },
        f
      );
    }
    return macro;
  }

  function calculateTotalGain(f: number): number {
    if (!localEq.enabled) return 0;
    let total = 0;
    for (const band of localEq.bands) {
      total += computeBiquadGainDb(band, f);
    }
    total += getMacroGainAt(f);
    return total;
  }

  function getBandNodeY(band: EqBand, height: number): number {
    if (!localEq.enabled) {
      return gainToY(0, height);
    }
    const effectiveGain = calculateTotalGain(band.freq_hz);
    return gainToY(effectiveGain, height);
  }

  export function draw() {
    if (!canvas) return;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    if (width <= 0 || height <= 0) return;
    const dpr = window.devicePixelRatio || 1;

    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    // 1. Matte Studio Background
    ctx.fillStyle = '#08090C';
    ctx.fillRect(0, 0, width, height);

    // 2. Frequency range regions
    const regions = [
      { name: 'SUB BASS', start: 20, end: 60, col: 'rgba(255, 255, 255, 0.008)' },
      { name: 'BASS', start: 60, end: 250, col: 'rgba(255, 255, 255, 0.016)' },
      { name: 'LOW MIDS', start: 250, end: 500, col: 'rgba(255, 255, 255, 0.008)' },
      { name: 'MID RANGE', start: 500, end: 2000, col: 'rgba(255, 255, 255, 0.016)' },
      { name: 'UPPER MIDS', start: 2000, end: 6000, col: 'rgba(255, 255, 255, 0.008)' },
      { name: 'HIGHS', start: 6000, end: 20000, col: 'rgba(255, 255, 255, 0.016)' },
    ];

    ctx.font = '700 9px "Inter", system-ui, sans-serif';
    ctx.textAlign = 'center';
    for (const r of regions) {
      const x1 = freqToX(r.start, width);
      const x2 = freqToX(r.end, width);
      ctx.fillStyle = r.col;
      ctx.fillRect(x1, 0, x2 - x1, height);

      ctx.fillStyle = '#64748B';
      ctx.fillText(r.name, (x1 + x2) / 2, 14);
    }

    // 3. Grid lines (dB lines)
    ctx.lineWidth = 1;
    ctx.fillStyle = '#64748B';
    ctx.font = '10px "JetBrains Mono", monospace';
    ctx.textAlign = 'right';

    for (const db of [-12, -6, 0, 6, 12]) {
      const y = gainToY(db, height);
      ctx.beginPath();
      ctx.strokeStyle = db === 0 ? '#2F3646' : '#1C2029';
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
      ctx.strokeStyle = '#1C2029';
      ctx.moveTo(x, 22);
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
        const barHeight = Math.max(3, signalNorm * (height * 0.48) * (0.35 + 0.65 * centerWeight));
        const barY = height - barHeight;

        ctx.fillStyle = `${accentColor}14`;
        ctx.fillRect(barX + 1, barY, barWidth - 2, barHeight);

        ctx.fillStyle = `${accentColor}38`;
        ctx.fillRect(barX + 1, barY, barWidth - 2, 1.5);
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

    ctx.strokeStyle = localEq.enabled ? accentColor : '#64748B';
    ctx.lineWidth = 2;
    ctx.stroke();

    ctx.lineTo(width, height / 2);
    ctx.lineTo(0, height / 2);
    ctx.closePath();
    const grad = ctx.createLinearGradient(0, 0, 0, height);
    grad.addColorStop(0, localEq.enabled ? `${accentColor}28` : 'rgba(100,116,139,0.12)');
    grad.addColorStop(1, `${accentColor}03`);
    ctx.fillStyle = grad;
    ctx.fill();

    // 5. Draw 10 draggable band nodes
    localEq.bands.forEach((band, index) => {
      const x = freqToX(band.freq_hz, width);
      const y = getBandNodeY(band, height);
      const isSelected = selectedBandIndex === index;

      if (isSelected) {
        ctx!.beginPath();
        ctx!.arc(x, y, 11, 0, Math.PI * 2);
        ctx!.strokeStyle = accentColor;
        ctx!.lineWidth = 1.5;
        ctx!.stroke();
      }

      ctx!.beginPath();
      ctx!.arc(x, y, 7, 0, Math.PI * 2);
      ctx!.fillStyle = !band.enabled ? '#262C38' : isSelected ? '#F8FAFC' : accentColor;
      ctx!.fill();
      ctx!.strokeStyle = '#08090C';
      ctx!.lineWidth = 1.5;
      ctx!.stroke();

      ctx!.fillStyle = isSelected && band.enabled ? '#08090C' : '#F8FAFC';
      ctx!.font = '800 8.5px "JetBrains Mono", monospace';
      ctx!.textAlign = 'center';
      ctx!.fillText(`${index + 1}`, x, y + 3);
    });
  }

  function handlePointerDown(e: PointerEvent) {
    if (!canvas || e.button !== 0) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    let found = -1;
    let minDistance = 16;
    for (let i = 0; i < localEq.bands.length; i++) {
      const bx = freqToX(localEq.bands[i].freq_hz, canvas.clientWidth);
      const by = getBandNodeY(localEq.bands[i], canvas.clientHeight);
      const dist = Math.hypot(x - bx, y - by);
      if (dist <= minDistance) {
        minDistance = dist;
        found = i;
      }
    }

    if (found !== -1) {
      selectedBandIndex = found;
      isDragging = true;
      canvas.setPointerCapture(e.pointerId);
      draw();
    }
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isDragging || selectedBandIndex === null || !canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const newFreq = Math.round(xToFreq(x, canvas.clientWidth));
    const targetGain = yToGain(y, canvas.clientHeight);
    const macro = getMacroGainAt(newFreq);
    const filter = localEq.bands[selectedBandIndex].filter_type;
    const isGainLocked = filter === 'high_pass' || filter === 'low_pass';

    localEq.bands[selectedBandIndex].freq_hz = Math.max(MIN_FREQ, Math.min(MAX_FREQ, newFreq));
    if (!isGainLocked) {
      const newGain = Math.round((targetGain - macro) * 10) / 10;
      localEq.bands[selectedBandIndex].gain_db = Math.max(MIN_GAIN, Math.min(MAX_GAIN, newGain));
    } else {
      localEq.bands[selectedBandIndex].gain_db = 0;
    }
    localEq.bands = [...localEq.bands];

    draw();
    emitChange(false);
  }

  function handlePointerUp(e: PointerEvent) {
    if (!isDragging) return;
    isDragging = false;
    try {
      canvas?.releasePointerCapture(e.pointerId);
    } catch {
      // Ignore
    }
    emitChange(true);
  }

  function handleDoubleClick(e: MouseEvent) {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    for (let i = 0; i < localEq.bands.length; i++) {
      const bx = freqToX(localEq.bands[i].freq_hz, canvas.clientWidth);
      const by = getBandNodeY(localEq.bands[i], canvas.clientHeight);
      if (Math.hypot(x - bx, y - by) <= 16) {
        localEq.bands[i].gain_db = 0.0;
        localEq.bands = [...localEq.bands];
        selectedBandIndex = i;
        draw();
        emitChange(true);
        break;
      }
    }
  }

  function handleWheel(e: WheelEvent) {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    let targetIndex = -1;
    for (let i = 0; i < localEq.bands.length; i++) {
      const bx = freqToX(localEq.bands[i].freq_hz, canvas.clientWidth);
      const by = getBandNodeY(localEq.bands[i], canvas.clientHeight);
      if (Math.hypot(x - bx, y - by) <= 18) {
        targetIndex = i;
        break;
      }
    }

    if (targetIndex === -1 && selectedBandIndex !== null) {
      const bx = freqToX(localEq.bands[selectedBandIndex].freq_hz, canvas.clientWidth);
      const by = getBandNodeY(localEq.bands[selectedBandIndex], canvas.clientHeight);
      if (Math.hypot(x - bx, y - by) <= 28) {
        targetIndex = selectedBandIndex;
      }
    }

    if (targetIndex !== -1) {
      e.preventDefault();
      selectedBandIndex = targetIndex;
      const delta = e.deltaY < 0 ? 0.2 : -0.2;
      const currentQ = localEq.bands[targetIndex].q ?? 1.0;
      const newQ = Math.round(Math.max(0.1, Math.min(10.0, currentQ + delta)) * 10) / 10;
      localEq.bands[targetIndex].q = newQ;
      localEq.bands = [...localEq.bands];
      draw();
      emitChange(false);
    }
  }

  const filterTypeOptions: SelectOption[] = [
    { value: 'peak', label: 'Peaking EQ' },
    { value: 'low_shelf', label: 'Low Shelf' },
    { value: 'high_shelf', label: 'High Shelf' },
    { value: 'high_pass', label: 'High Pass' },
    { value: 'low_pass', label: 'Low Pass' },
    { value: 'notch', label: 'Notch Filter' },
  ];

  function updateSelectedBand(patch: Partial<EqBand>, immediate = true) {
    if (!localEq.bands[selectedBandIndex]) return;
    localEq.bands[selectedBandIndex] = {
      ...localEq.bands[selectedBandIndex],
      ...patch,
    };
    localEq.bands = [...localEq.bands];
    draw();
    emitChange(immediate);
  }

  function handleFilterTypeChange(val: string) {
    const ft = val as FilterType;
    const patch: Partial<EqBand> = { filter_type: ft };
    if (ft === 'high_pass' || ft === 'low_pass') {
      patch.gain_db = 0;
    }
    updateSelectedBand(patch, true);
  }

  function handleFreqChange(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const raw = parseFloat(input.value);
    if (isNaN(raw)) {
      input.value = String(Math.round(activeBand.freq_hz));
      return;
    }
    const clamped = Math.max(MIN_FREQ, Math.min(MAX_FREQ, Math.round(raw)));
    input.value = String(clamped);
    updateSelectedBand({ freq_hz: clamped }, true);
  }

  function handleGainChange(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const raw = parseFloat(input.value);
    if (isNaN(raw)) {
      input.value = String(activeBand.gain_db);
      return;
    }
    const clamped = Math.max(MIN_GAIN, Math.min(MAX_GAIN, Math.round(raw * 10) / 10));
    input.value = String(clamped);
    updateSelectedBand({ gain_db: clamped }, true);
  }

  function handleQChange(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const raw = parseFloat(input.value);
    if (isNaN(raw)) {
      input.value = String(activeBand.q);
      return;
    }
    const clamped = Math.max(0.1, Math.min(10, Math.round(raw * 10) / 10));
    input.value = String(clamped);
    updateSelectedBand({ q: clamped }, true);
  }

  onMount(() => {
    draw();
    window.addEventListener('resize', draw);
    return () => window.removeEventListener('resize', draw);
  });

  onDestroy(() => {
    if (throttleTimer) clearTimeout(throttleTimer);
  });

  $: if (accentColor || meter) {
    draw();
  }

  $: activeBand = localEq.bands[selectedBandIndex] || localEq.bands[0];
  $: isGainLocked = activeBand ? (activeBand.filter_type === 'high_pass' || activeBand.filter_type === 'low_pass') : false;
</script>

<div class="eq-component-wrapper">
  <!-- SteelSeries GG Sonar Compact Single-Band Inline Inspector Bar -->
  {#if activeBand}
    <div class="sonar-band-bar">
      <div class="band-selector-pills" role="tablist" aria-label="EQ Band Selector">
        <span class="bar-section-label">BAND</span>
        {#each localEq.bands as band, i}
          <button
            type="button"
            class="band-num-pill tabular-nums"
            class:selected={selectedBandIndex === i}
            class:disabled-band={!band.enabled}
            style="--eq-accent: {accentColor};"
            on:click={() => {
              selectedBandIndex = i;
              draw();
            }}
            title="Band {i + 1}: {band.freq_hz} Hz ({band.gain_db >= 0 ? '+' : ''}{band.gain_db} dB)"
          >
            {i + 1}
          </button>
        {/each}
      </div>

      <div class="band-inline-controls">
        <div class="inline-field filter-field">
          <span class="field-label">FILTER</span>
          <div class="filter-select-wrap">
            <CustomSelect
              size="sm"
              value={activeBand.filter_type}
              options={filterTypeOptions}
              onChange={handleFilterTypeChange}
            />
          </div>
        </div>

        <div class="inline-field">
          <span class="field-label">FREQ</span>
          <div class="num-input-wrap">
            <input
              type="number"
              min="20"
              max="20000"
              step="1"
              class="inline-num-input tabular-nums"
              value={Math.round(activeBand.freq_hz)}
              on:change={handleFreqChange}
              on:keydown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
            />
            <span class="unit">Hz</span>
          </div>
        </div>

        <div class="inline-field" class:is-disabled-field={isGainLocked}>
          <span class="field-label">GAIN</span>
          <div class="num-input-wrap">
            <input
              type="number"
              min="-12"
              max="12"
              step="0.5"
              class="inline-num-input tabular-nums"
              disabled={isGainLocked}
              value={isGainLocked ? 0 : activeBand.gain_db}
              on:change={handleGainChange}
              on:keydown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
            />
            <span class="unit">dB</span>
          </div>
        </div>

        <div class="inline-field">
          <span class="field-label">Q FACTOR</span>
          <div class="num-input-wrap q-wrap">
            <input
              type="number"
              min="0.1"
              max="10"
              step="0.1"
              class="inline-num-input tabular-nums"
              value={activeBand.q}
              on:change={handleQChange}
              on:keydown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
            />
          </div>
        </div>

        <button
          type="button"
          class="band-state-btn"
          class:active={activeBand.enabled}
          style="--eq-accent: {accentColor};"
          on:click={() => updateSelectedBand({ enabled: !activeBand.enabled }, true)}
          title={activeBand.enabled ? 'Disable Band' : 'Enable Band'}
        >
          {activeBand.enabled ? 'ACTIVE' : 'BYPASS'}
        </button>
      </div>
    </div>
  {/if}

  <div class="eq-graph-container">
    <canvas
      bind:this={canvas}
      on:pointerdown={handlePointerDown}
      on:pointermove={handlePointerMove}
      on:pointerup={handlePointerUp}
      on:pointercancel={handlePointerUp}
      on:dblclick={handleDoubleClick}
      on:wheel={handleWheel}
    ></canvas>
  </div>
</div>

<style>
  .eq-component-wrapper {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 100%;
    font-family: var(--font-sans);
  }

  .sonar-band-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px;
    background: var(--color-surface-2, #171B22);
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-sm, 4px);
    padding: 8px 12px;
  }

  .band-selector-pills {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .bar-section-label {
    font-size: 9px;
    font-weight: 800;
    color: var(--color-text-muted, #64748B);
    letter-spacing: 0.6px;
    margin-inline-end: 4px;
  }

  .band-num-pill {
    width: 24px;
    height: 24px;
    border-radius: var(--radius-xs, 3px);
    background: var(--color-surface-3, #1E232D);
    border: 1px solid var(--color-border-default, #232834);
    color: var(--color-text-secondary, #94A3B8);
    font-size: 10px;
    font-weight: 800;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition-fast);
  }

  .band-num-pill:hover {
    color: var(--color-text-primary, #F8FAFC);
    border-color: var(--color-border-bright, #2F3646);
  }

  .band-num-pill.selected {
    background: var(--eq-accent, #FF5200);
    border-color: var(--eq-accent, #FF5200);
    color: #08090C;
  }

  .band-num-pill.disabled-band {
    opacity: 0.4;
  }

  .band-inline-controls {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .inline-field {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .inline-field.is-disabled-field {
    opacity: 0.35;
    pointer-events: none;
  }

  .field-label {
    font-size: 9px;
    font-weight: 800;
    color: var(--color-text-muted, #64748B);
    letter-spacing: 0.5px;
  }

  .filter-select-wrap {
    width: 122px;
  }

  .num-input-wrap {
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--color-neutral-950, #08090C);
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-xs, 3px);
    padding: 3px 6px;
    width: 78px;
  }

  .num-input-wrap.q-wrap {
    width: 56px;
  }

  .num-input-wrap:focus-within {
    border-color: var(--color-border-bright, #2F3646);
  }

  .inline-num-input {
    width: 100%;
    background: transparent;
    border: none;
    color: var(--color-text-primary, #F8FAFC);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
    outline: none;
    padding: 0;
  }

  .unit {
    font-size: 9px;
    color: var(--color-text-muted, #64748B);
    font-family: var(--font-mono);
  }

  .band-state-btn {
    height: 24px;
    padding: 0 8px;
    border-radius: var(--radius-xs, 3px);
    font-size: 9px;
    font-weight: 800;
    font-family: var(--font-mono);
    border: 1px solid var(--color-border-default, #232834);
    background: var(--color-surface-3, #1E232D);
    color: var(--color-text-muted, #64748B);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .band-state-btn.active {
    background: rgba(0, 229, 153, 0.12);
    color: var(--color-channel-game, #00E599);
    border-color: rgba(0, 229, 153, 0.35);
  }

  .eq-graph-container {
    position: relative;
    width: 100%;
    height: 220px;
    background: var(--color-neutral-950, #08090C);
    border-radius: var(--radius-sm, 4px);
    overflow: hidden;
    border: 1px solid var(--color-border-default, #232834);
  }

  canvas {
    width: 100%;
    height: 100%;
    display: block;
    cursor: crosshair;
    touch-action: none;
  }
</style>
