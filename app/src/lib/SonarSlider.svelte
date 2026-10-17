<script lang="ts">
  export let value: number = 0;
  export let min: number = 0;
  export let max: number = 100;
  export let step: number = 1;
  export let centerDetent: boolean = false;
  export let defaultValue: number = centerDetent ? 0 : 0;
  export let unit: string = '';
  export let label: string = '';
  export let accentColor: string = '#FF5200';
  export let disabled: boolean = false;
  export let size: 'sm' | 'md' = 'md';
  export let onChange: ((val: number) => void) | undefined = undefined;

  let trackEl: HTMLElement | null = null;
  let isDragging = false;

  $: range = max - min;
  $: normalizedPos = range > 0 ? Math.max(0, Math.min(1, (value - min) / range)) : 0;
  $: thumbPercent = normalizedPos * 100;

  // Center detent fill calculations
  $: centerNorm = range > 0 ? (0 - min) / range : 0.5;
  $: centerPercent = centerNorm * 100;
  $: fillLeft = centerDetent
    ? Math.min(centerPercent, thumbPercent)
    : 0;
  $: fillWidth = centerDetent
    ? Math.abs(thumbPercent - centerPercent)
    : thumbPercent;

  function updateFromClientX(clientX: number) {
    if (!trackEl || disabled) return;
    const rect = trackEl.getBoundingClientRect();
    if (rect.width <= 0) return;

    const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    let rawVal = min + ratio * range;

    if (step > 0) {
      rawVal = Math.round(rawVal / step) * step;
    }

    // Snap to 0 if centerDetent and within 3% range
    if (centerDetent && Math.abs(rawVal) <= Math.abs(range * 0.03)) {
      rawVal = 0;
    }

    rawVal = Math.max(min, Math.min(max, rawVal));

    if (rawVal !== value) {
      value = rawVal;
      if (onChange) onChange(value);
    }
  }

  function handlePointerDown(e: PointerEvent) {
    if (disabled || e.button !== 0) return;
    isDragging = true;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
    updateFromClientX(e.clientX);
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isDragging) return;
    updateFromClientX(e.clientX);
  }

  function handlePointerUp(e: PointerEvent) {
    if (!isDragging) return;
    isDragging = false;
    try {
      (e.target as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // Ignored
    }
  }

  function handleDoubleClick() {
    if (disabled) return;
    value = defaultValue;
    if (onChange) onChange(value);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (disabled) return;
    let delta = 0;
    if (e.key === 'ArrowRight' || e.key === 'ArrowUp') delta = e.shiftKey ? step * 5 : step;
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') delta = e.shiftKey ? -step * 5 : -step;
    else if (e.key === 'Home') {
      value = min;
      if (onChange) onChange(value);
      return;
    } else if (e.key === 'End') {
      value = max;
      if (onChange) onChange(value);
      return;
    }

    if (delta !== 0) {
      e.preventDefault();
      const newVal = Math.max(min, Math.min(max, value + delta));
      if (newVal !== value) {
        value = newVal;
        if (onChange) onChange(value);
      }
    }
  }

  function formatValue(v: number): string {
    let str = (step < 1 ? v.toFixed(1) : v.toString());
    if (unit === 'dB' && v > 0) str = '+' + str;
    return unit ? `${str} ${unit}` : str;
  }
</script>

<div class="sonar-slider-container {size}" class:disabled class:is-dragging={isDragging}>
  {#if label || $$slots.label}
    <div class="slider-header">
      <span class="slider-label">
        {#if $$slots.label}<slot name="label" />{:else}{label}{/if}
      </span>
      <span class="slider-value" style="color: {accentColor};">
        {formatValue(value)}
      </span>
    </div>
  {/if}

  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div
    class="slider-track-rail"
    bind:this={trackEl}
    on:pointerdown={handlePointerDown}
    on:pointermove={handlePointerMove}
    on:pointerup={handlePointerUp}
    on:pointercancel={handlePointerUp}
    on:dblclick={handleDoubleClick}
    on:keydown={handleKeyDown}
    role="slider"
    tabindex="0"
    aria-label={label || 'Slider'}
    aria-valuemin={min}
    aria-valuemax={max}
    aria-valuenow={value}
    aria-valuetext={formatValue(value)}
  >
    <!-- Background Rail Slot -->
    <div class="rail-slot">
      <!-- Active Fill Track -->
      <div
        class="rail-fill"
        style="left: {fillLeft}%; width: {fillWidth}%; background-color: {accentColor};"
      ></div>

      <!-- Center Detent Notch Marker -->
      {#if centerDetent}
        <div class="center-detent-marker" style="left: {centerPercent}%;"></div>
      {/if}
    </div>

    <!-- Tactile Thumb -->
    <div
      class="slider-thumb"
      style="left: {thumbPercent}%;"
    >
      <div class="thumb-groove" style="background-color: {accentColor};"></div>
    </div>
  </div>
</div>

<style>
  .sonar-slider-container {
    display: flex;
    flex-direction: column;
    gap: 5px;
    width: 100%;
    user-select: none;
    font-family: var(--font-sans);
  }

  .slider-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.775rem;
  }

  .slider-label {
    color: var(--color-neutral-300, #cbd5e1);
    font-weight: 500;
  }

  .slider-value {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .slider-track-rail {
    position: relative;
    width: 100%;
    height: 20px;
    display: flex;
    align-items: center;
    cursor: pointer;
    touch-action: none;
    outline: none;
  }

  .slider-track-rail:focus-visible .slider-thumb {
    outline: 2px solid var(--color-accent-brand, #ff5200);
    outline-offset: 2px;
  }

  .rail-slot {
    position: relative;
    width: 100%;
    height: 4px;
    background: var(--color-neutral-950, #07090d);
    border: 1px solid var(--color-border-default, #1f2637);
    border-radius: 2px;
    overflow: hidden;
  }

  .rail-fill {
    position: absolute;
    top: 0;
    bottom: 0;
    border-radius: 1px;
    opacity: 0.9;
  }

  .center-detent-marker {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    transform: translateX(-50%);
    background: var(--color-neutral-400, #94a3b8);
    z-index: 1;
  }

  .slider-thumb {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 14px;
    height: 18px;
    background: #e2e8f0;
    border: 1px solid #0f172a;
    border-radius: 3px;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.5), inset 0 1px 0 rgba(255, 255, 255, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    z-index: 2;
    transition: transform var(--transition-fast, 120ms);
  }

  .is-dragging .slider-thumb {
    background: #ffffff;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.7);
    transform: translate(-50%, -50%) scale(1.08);
  }

  .thumb-groove {
    width: 2px;
    height: 10px;
    border-radius: 1px;
  }

  /* Sizes */
  .sm .slider-track-rail {
    height: 16px;
  }

  .sm .slider-thumb {
    width: 11px;
    height: 14px;
  }

  .sm .thumb-groove {
    height: 8px;
  }

  .disabled {
    opacity: 0.45;
    pointer-events: none;
  }
</style>
