<script lang="ts">
  import type { EqConfig, EqBand } from '../types';

  export let eqConfig: EqConfig | undefined = undefined;
  export let accentColor: string = '#10B981';
  export let height: number = 42;

  const WIDTH = 160;
  const HEIGHT = 42;
  const MID_Y = HEIGHT / 2; // 0 dB line
  const DB_RANGE = 24; // -12 dB to +12 dB range
  const PIXELS_PER_DB = (HEIGHT * 0.42) / (DB_RANGE / 2);

  // Compute logarithmic frequency to X coordinate (20 Hz -> 0, 20000 Hz -> WIDTH)
  function freqToX(freq: number): number {
    const minLog = Math.log10(20);
    const maxLog = Math.log10(20000);
    const curLog = Math.log10(Math.max(20, Math.min(20000, freq)));
    return ((curLog - minLog) / (maxLog - minLog)) * WIDTH;
  }

  function xToFreq(x: number): number {
    const minLog = Math.log10(20);
    const maxLog = Math.log10(20000);
    const curLog = minLog + (x / WIDTH) * (maxLog - minLog);
    return Math.pow(10, curLog);
  }

  // Calculate dB gain contribution of a band at frequency f using RBJ peaking formula
  function evalBand(band: EqBand, f: number): number {
    if (!band.enabled || band.gain_db === 0) return 0;
    const f0 = band.freq_hz;
    const gain = band.gain_db;
    const q = Math.max(0.2, band.q);

    if (band.filter_type === 'low_shelf') {
      if (f <= f0) return gain;
      const ratio = f / f0;
      return gain / (1 + Math.pow(ratio, 2 * q));
    } else if (band.filter_type === 'high_shelf') {
      if (f >= f0) return gain;
      const ratio = f0 / f;
      return gain / (1 + Math.pow(ratio, 2 * q));
    } else {
      // Peaking / parametric
      const w = Math.log2(f / f0);
      const bandwidth = 1.0 / q;
      const factor = Math.exp(-0.5 * Math.pow(w / bandwidth, 2));
      return gain * factor;
    }
  }

  // Compute total EQ curve points
  $: pathData = (() => {
    if (!eqConfig || !eqConfig.enabled || !eqConfig.bands || eqConfig.bands.length === 0) {
      return {
        line: `M 0,${MID_Y} L ${WIDTH},${MID_Y}`,
        area: `M 0,${MID_Y} L ${WIDTH},${MID_Y} L ${WIDTH},${MID_Y} L 0,${MID_Y} Z`,
        isFlat: true,
      };
    }

    const points: [number, number][] = [];
    const NUM_POINTS = 32;

    for (let i = 0; i <= NUM_POINTS; i++) {
      const x = (i / NUM_POINTS) * WIDTH;
      const freq = xToFreq(x);

      let totalGainDb = 0;
      for (const band of eqConfig.bands) {
        totalGainDb += evalBand(band, freq);
      }

      // Macro tone offsets
      if (eqConfig.bass_db && freq < 250) {
        totalGainDb += eqConfig.bass_db * (1 - freq / 250);
      }
      if (eqConfig.treble_db && freq > 4000) {
        totalGainDb += eqConfig.treble_db * ((freq - 4000) / 16000);
      }

      const y = Math.max(2, Math.min(HEIGHT - 2, MID_Y - totalGainDb * PIXELS_PER_DB));
      points.push([x, y]);
    }

    // Build smooth SVG path
    let linePath = `M ${points[0][0].toFixed(1)},${points[0][1].toFixed(1)}`;
    for (let i = 1; i < points.length; i++) {
      linePath += ` L ${points[i][0].toFixed(1)},${points[i][1].toFixed(1)}`;
    }

    const areaPath = `${linePath} L ${WIDTH},${MID_Y} L 0,${MID_Y} Z`;

    return {
      line: linePath,
      area: areaPath,
      isFlat: false,
    };
  })();
</script>

<div class="mini-eq-card" style="height: {height}px;">
  <svg
    viewBox="0 0 {WIDTH} {HEIGHT}"
    preserveAspectRatio="none"
    class="mini-eq-svg"
  >
    <defs>
      <linearGradient id="grad-{accentColor.replace('#', '')}" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color={accentColor} stop-opacity="0.35" />
        <stop offset="100%" stop-color={accentColor} stop-opacity="0.02" />
      </linearGradient>
    </defs>

    <!-- Grid lines -->
    <line x1="0" y1={MID_Y} x2={WIDTH} y2={MID_Y} class="grid-center-line" />

    <!-- Fill area under curve -->
    {#if !pathData.isFlat}
      <path
        d={pathData.area}
        fill="url(#grad-{accentColor.replace('#', '')})"
      />
    {/if}

    <!-- Curve line -->
    <path
      d={pathData.line}
      fill="none"
      stroke={pathData.isFlat ? 'var(--color-neutral-600, #333d56)' : accentColor}
      stroke-width={pathData.isFlat ? '1' : '1.8'}
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>

  {#if pathData.isFlat}
    <span class="flat-badge">FLAT</span>
  {/if}
</div>

<style>
  .mini-eq-card {
    position: relative;
    width: 100%;
    background: var(--color-neutral-950, #07090d);
    border: 1px solid var(--color-border-subtle, #1c2230);
    border-radius: var(--radius-sm, 4px);
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .mini-eq-svg {
    width: 100%;
    height: 100%;
    display: block;
  }

  .grid-center-line {
    stroke: var(--color-border-subtle, #1c2230);
    stroke-width: 1;
    stroke-dasharray: 2 3;
  }

  .flat-badge {
    position: absolute;
    font-size: 0.6rem;
    font-weight: 700;
    color: var(--color-neutral-500, #64748b);
    letter-spacing: 0.06em;
    pointer-events: none;
    font-family: var(--font-mono);
  }
</style>
