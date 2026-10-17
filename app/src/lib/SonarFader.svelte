<script context="module" lang="ts">
  export function volumeToDb(vol: number): string {
    if (vol <= 0) return '-\u221E dB';
    const db = 20 * Math.log10(vol / 100);
    return (db >= 0 ? '+' : '') + db.toFixed(1) + ' dB';
  }
</script>

<script lang="ts">
  import type { ChannelMeter } from '../types';

  export let value: number = 100; // 0 to 150
  export let meter: ChannelMeter | undefined = undefined;
  export let accentColor: string = '#FF5200';
  export let disabled: boolean = false;
  export let onChange: ((val: number) => void) | undefined = undefined;

  let trackEl: HTMLElement | null = null;
  let isDragging = false;

  $: displayDb = volumeToDb(value);
  $: sliderPositionPct = Math.min(100, Math.max(0, (value / 150) * 100));

  // Meter calculations: -60 dBFS to 0 dBFS mapped to 0% - 100%
  $: peakNorm = meter ? Math.max(0, Math.min(100, ((meter.peak_dbfs + 60) / 60) * 100)) : 0;
  $: rmsNorm = meter ? Math.max(0, Math.min(100, ((meter.rms_dbfs + 60) / 60) * 100)) : 0;

  function updateFromPointer(clientY: number) {
    if (!trackEl || disabled) return;
    const rect = trackEl.getBoundingClientRect();
    const bottom = rect.bottom;
    const height = rect.height;
    if (height <= 0) return;

    const offset = bottom - clientY;
    const ratio = Math.max(0, Math.min(1, offset / height));
    const newVal = Math.round(ratio * 150);

    if (newVal !== value) {
      value = newVal;
      if (onChange) onChange(value);
    }
  }

  function handlePointerDown(e: PointerEvent) {
    if (disabled || e.button !== 0) return;
    isDragging = true;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
    updateFromPointer(e.clientY);
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isDragging) return;
    updateFromPointer(e.clientY);
  }

  function handlePointerUp(e: PointerEvent) {
    if (!isDragging) return;
    isDragging = false;
    try {
      (e.target as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // Ignored if capture already released
    }
  }

  function handleDoubleClick() {
    if (disabled) return;
    // Unity gain reset (0 dB / 100%)
    value = 100;
    if (onChange) onChange(value);
  }

  function handleWheel(e: WheelEvent) {
    if (disabled) return;
    e.preventDefault();
    const delta = e.deltaY < 0 ? 2 : -2;
    const newVal = Math.max(0, Math.min(150, value + delta));
    if (newVal !== value) {
      value = newVal;
      if (onChange) onChange(value);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (disabled) return;
    let step = 0;
    if (e.key === 'ArrowUp' || e.key === 'ArrowRight') step = e.shiftKey ? 5 : 1;
    else if (e.key === 'ArrowDown' || e.key === 'ArrowLeft') step = e.shiftKey ? -5 : -1;
    else if (e.key === 'Home') {
      value = 150;
      if (onChange) onChange(value);
      return;
    } else if (e.key === 'End') {
      value = 0;
      if (onChange) onChange(value);
      return;
    }

    if (step !== 0) {
      e.preventDefault();
      const newVal = Math.max(0, Math.min(150, value + step));
      if (newVal !== value) {
        value = newVal;
        if (onChange) onChange(value);
      }
    }
  }
</script>

<div
  class="sonar-fader"
  class:disabled
  class:is-dragging={isDragging}
  on:wheel={handleWheel}
>
  <!-- Fader Console Bay -->
  <div class="fader-bay">
    <!-- Scale Ticks Left -->
    <div class="scale-ticks">
      <div class="tick-label top" style="bottom: 100%;">+3.5</div>
      <div class="tick-label zero" style="bottom: 66.7%;">0 dB</div>
      <div class="tick-label minus6" style="bottom: 33.3%;">-6</div>
      <div class="tick-label minus12" style="bottom: 16.7%;">-12</div>
      <div class="tick-label bottom" style="bottom: 0%;">-\u221E</div>
    </div>

    <!-- Active Fader Rail & Thumb -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div
      class="fader-track-rail"
      bind:this={trackEl}
      on:pointerdown={handlePointerDown}
      on:pointermove={handlePointerMove}
      on:pointerup={handlePointerUp}
      on:pointercancel={handlePointerUp}
      on:dblclick={handleDoubleClick}
      on:keydown={handleKeyDown}
      role="slider"
      tabindex="0"
      aria-label="Channel Volume Fader"
      aria-valuemin="0"
      aria-valuemax="150"
      aria-valuenow={value}
      aria-valuetext="{value}% ({displayDb})"
    >
      <!-- Unity Gain 0dB Guideline -->
      <div class="unity-gain-line" style="bottom: 66.7%;"></div>

      <!-- Center Slot Track -->
      <div class="rail-slot">
        <div
          class="rail-fill"
          style="height: {sliderPositionPct}%; background-color: {accentColor};"
        ></div>
      </div>

      <!-- Tactile Hardware Thumb -->
      <div
        class="fader-thumb"
        style="bottom: calc({sliderPositionPct}% - 9px);"
      >
        <div class="thumb-grip-line" style="background-color: {accentColor};"></div>
      </div>
    </div>

    <!-- Live LED Segment Meter -->
    <div class="fader-meter">
      <div class="meter-channel-bar">
        <!-- RMS Level Bar (Solid) -->
        <div class="meter-level rms" style="height: {rmsNorm}%;"></div>
        <!-- Peak Level Line (Bright Top Marker) -->
        {#if peakNorm > 1}
          <div class="meter-peak-marker" style="bottom: {peakNorm}%;"></div>
        {/if}
      </div>
      <!-- Zero dBFS Clip Warning LED -->
      <div class="clip-indicator" class:clipped={peakNorm >= 98}></div>
    </div>
  </div>

  <!-- Readout Display -->
  <div class="readout-block">
    <span class="db-val">{displayDb}</span>
    <span class="pct-val">{value}%</span>
  </div>
</div>

<style>
  .sonar-fader {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 64px;
    user-select: none;
    font-family: var(--font-sans);
  }

  .fader-bay {
    display: flex;
    align-items: stretch;
    justify-content: center;
    gap: 7px;
    height: 180px;
    position: relative;
    padding: 6px 0;
  }

  /* Scale Ticks */
  .scale-ticks {
    position: relative;
    width: 22px;
    height: 100%;
  }

  .tick-label {
    position: absolute;
    right: 2px;
    transform: translateY(50%);
    font-size: 0.625rem;
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    color: var(--color-neutral-500, #64748b);
    line-height: 1;
    pointer-events: none;
  }

  .tick-label.zero {
    color: var(--color-neutral-200, #e2e8f0);
    font-weight: 700;
  }

  /* Fader Rail */
  .fader-track-rail {
    position: relative;
    width: 24px;
    height: 100%;
    cursor: pointer;
    touch-action: none;
    outline: none;
  }

  .fader-track-rail:focus-visible .fader-thumb {
    outline: 2px solid var(--color-accent-brand, #ff5200);
    outline-offset: 2px;
  }

  .unity-gain-line {
    position: absolute;
    left: -2px;
    right: -2px;
    height: 1px;
    background: var(--color-neutral-600, #333d56);
    pointer-events: none;
    z-index: 1;
  }

  .rail-slot {
    position: absolute;
    left: 50%;
    top: 0;
    bottom: 0;
    width: 4px;
    transform: translateX(-50%);
    background: var(--color-neutral-950, #07090d);
    border: 1px solid var(--color-border-default, #1f2637);
    border-radius: 2px;
    overflow: hidden;
  }

  .rail-fill {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    opacity: 0.35;
    pointer-events: none;
  }

  /* Tactile Hardware Thumb */
  .fader-thumb {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    width: 24px;
    height: 18px;
    background: #e2e8f0;
    border: 1px solid #0f172a;
    border-radius: 3px;
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.6), inset 0 1px 0 rgba(255, 255, 255, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 5;
    pointer-events: none;
  }

  .is-dragging .fader-thumb {
    background: #ffffff;
    box-shadow: 0 4px 10px rgba(0, 0, 0, 0.7);
  }

  .thumb-grip-line {
    width: 14px;
    height: 2px;
    border-radius: 1px;
  }

  /* Meter Bar */
  .fader-meter {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    width: 7px;
    height: 100%;
  }

  .meter-channel-bar {
    position: relative;
    width: 7px;
    flex: 1;
    background: var(--color-neutral-950, #07090d);
    border: 1px solid var(--color-border-subtle, #1c2230);
    border-radius: 2px;
    overflow: hidden;
  }

  .meter-level.rms {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    background: linear-gradient(
      to top,
      #10b981 0%,
      #10b981 70%,
      #f59e0b 88%,
      #ef4444 100%
    );
    border-radius: 1px;
    transition: height 50ms linear;
  }

  .meter-peak-marker {
    position: absolute;
    left: 0;
    right: 0;
    height: 2px;
    background: #ffffff;
    box-shadow: 0 0 4px rgba(255, 255, 255, 0.9);
    z-index: 2;
  }

  .clip-indicator {
    width: 6px;
    height: 4px;
    border-radius: 1px;
    background: #331515;
    transition: background 60ms ease-out;
  }

  .clip-indicator.clipped {
    background: #ef4444;
    box-shadow: 0 0 6px #ef4444;
  }

  /* Readout Display */
  .readout-block {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-top: 6px;
    gap: 1px;
  }

  .db-val {
    font-size: 0.725rem;
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    color: var(--color-neutral-100, #f1f5f9);
    letter-spacing: -0.02em;
  }

  .pct-val {
    font-size: 0.65rem;
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    color: var(--color-neutral-500, #64748b);
  }

  .disabled {
    opacity: 0.4;
    pointer-events: none;
  }
</style>
