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
  let localValue = value;

  $: if (!isDragging) {
    localValue = value;
  }

  $: range = max - min;
  $: normalizedPos = range > 0 ? Math.max(0, Math.min(1, (localValue - min) / range)) : 0;
  $: thumbPercent = normalizedPos * 100;

  // Center detent fill calculations
  $: centerNorm = range > 0 ? (0 - min) / range : 0.5;
  $: centerPercent = centerNorm * 100;
  $: fillLeft = centerDetent ? Math.min(centerPercent, thumbPercent) : 0;
  $: fillWidth = centerDetent ? Math.abs(thumbPercent - centerPercent) : thumbPercent;

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

    if (rawVal !== localValue) {
      localValue = rawVal;
      value = rawVal;
      if (onChange) onChange(rawVal);
    }
  }

  function handlePointerDown(e: PointerEvent) {
    if (disabled || e.button !== 0) return;
    isDragging = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
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
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // Ignored
    }
  }

  function handleDoubleClick() {
    if (disabled) return;
    localValue = defaultValue;
    value = defaultValue;
    if (onChange) onChange(defaultValue);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (disabled) return;
    let delta = 0;
    if (e.key === 'ArrowRight' || e.key === 'ArrowUp') delta = e.shiftKey ? step * 5 : step;
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') delta = e.shiftKey ? -step * 5 : -step;
    else if (e.key === 'Home') {
      localValue = min;
      value = min;
      if (onChange) onChange(min);
      return;
    } else if (e.key === 'End') {
      localValue = max;
      value = max;
      if (onChange) onChange(max);
      return;
    }

    if (delta !== 0) {
      e.preventDefault();
      const newVal = Math.max(min, Math.min(max, localValue + delta));
      if (newVal !== localValue) {
        localValue = newVal;
        value = newVal;
        if (onChange) onChange(newVal);
      }
    }
  }

  function formatValue(v: number): string {
    let str = step < 1 ? v.toFixed(1) : v.toString();
    if (unit.trim() === 'dB' && v > 0) str = '+' + str;
    return unit ? `${str} ${unit.trim()}` : str;
  }
</script>

<div class="sonar-slider-container {size}" class:disabled class:is-dragging={isDragging}>
  {#if label || $$slots.label}
    <div class="slider-header">
      <span class="slider-label">
        {#if $$slots.label}<slot name="label" />{:else}{label}{/if}
      </span>
      <span class="slider-value" style="color: {accentColor};">
        {formatValue(localValue)}
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
    aria-valuenow={localValue}
    aria-valuetext={formatValue(localValue)}
  >
    <div class="rail-slot">
      <div
        class="rail-fill"
        style="left: {fillLeft}%; width: {fillWidth}%; background-color: {accentColor};"
      ></div>

      {#if centerDetent}
        <div class="center-detent-marker" style="left: {centerPercent}%;"></div>
      {/if}
    </div>

    <div class="slider-thumb" style="left: {thumbPercent}%;">
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
    font-size: 0.75rem;
  }

  .slider-label {
    color: var(--color-neutral-300, #CBD5E1);
    font-weight: 600;
  }

  .slider-value {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-size: 0.725rem;
    font-weight: 700;
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
    outline: 2px solid var(--color-accent-brand, #FF5200);
    outline-offset: 2px;
  }

  .rail-slot {
    position: relative;
    width: 100%;
    height: 4px;
    background: var(--color-neutral-950, #08090C);
    border: 1px solid var(--color-border-default, #232834);
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
    background: var(--color-neutral-400, #94A3B8);
    z-index: 1;
  }

  .slider-thumb {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 14px;
    height: 18px;
    background: var(--color-neutral-200, #E2E8F0);
    border: 1px solid var(--color-neutral-950, #08090C);
    border-radius: 3px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    z-index: 2;
  }

  .is-dragging .slider-thumb {
    background: #FFFFFF;
  }

  .thumb-groove {
    width: 2px;
    height: 10px;
    border-radius: 1px;
  }

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
