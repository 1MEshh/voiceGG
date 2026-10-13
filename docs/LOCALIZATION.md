# VoiceGG Localization & Internationalization (i18n) Architecture

VoiceGG features first-class internationalization with native bidirectional support for **English (LTR)** and **Arabic (RTL)**.

---

## 1. Core Principles & Philosophy

### 1.1 Authentic Pro-Audio Terminology
Audio engineering terminology is frequently translated poorly by automated machine translation systems, resulting in clumsy phrases that alienate sound engineers and gamers. In VoiceGG, every Arabic term has been hand-crafted to align with established professional broadcasting and audio engineering standards:

| English Key | Accurate Pro-Audio Arabic | Avoided Machine Translation |
|:---|:---|:---|
| Multi-Channel Mixer | مكساج القنوات المتعددة | خلاط القنوات |
| Live Voice Monitoring | مراقبة الصوت المباشرة | مراقبة الصوت الحي |
| Noise Gate | بوابة الضوضاء | بوابة الإزعاج |
| Compressor | الضاغط الصوتي | ضاغط الصوت |
| Parametric Equalizer | معادل الصوت البارامتري | موازن الترددات |
| Gain Reduction | تقليل الكسب | تخفيض الصوت |
| Hysteresis Threshold | عتبة التباطؤ الحركي | حد التردد |
| ChatMix Balance | موازنة المحادثة والألعاب | خلط الدردشة |
| Panic Reset | إعادة ضبط الصوت الطارئة | إعادة ضبط الذعر |

### 1.2 Pro-Audio Layout Rules (Directional Symmetry vs Standard Consoles)
While textual layouts, headers, modals, and settings panels mirror completely when switching to Arabic (`dir="rtl"`), VoiceGG enforces one critical rule:
- **The Channel Strips Mixer (`.channel-strips`) remains strictly LTR (`direction: ltr !important`)**:
  In professional audio consoles worldwide, channel strip order (`Master` $\to$ `Game` $\to$ `Chat` $\to$ `Media` $\to$ `Aux` $\to$ `Mic`) is universally standardized. Inverting physical fader columns confuses muscle memory during intense gaming matches. While the channel labels, sliders, and badges are rendered in Arabic with authentic typography, the physical fader sequencing is preserved.

---

## 2. Typography & Font Architecture

VoiceGG integrates a custom typography hierarchy tailored for legibility and high contrast:

```css
:root {
  --font-sans: 'thmanyah sans', 'IBM Plex Sans Arabic', 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
  --font-mono: 'JetBrains Mono', monospace;
  --tabular-nums: tabular-nums;
}
```

- **Arabic Typography**: Prioritizes **Thmanyah (`ثمانية`)** and **IBM Plex Sans Arabic** for clean, modern Arabic glyph rendering without clipping or strange ligatures.
- **English Typography**: **Inter** for clean geometric clarity.
- **Meters & Numerical Values**: **JetBrains Mono** with `font-variant-numeric: tabular-nums;`. This prevents visual jitter when dBFS decibel meters and percentages update at 60 Hz.

---

## 3. Automated Parity Verification

To eliminate runtime translation errors or untranslated keys, VoiceGG includes an automated test in `crates/voicegg-core/tests/i18n_test.rs`:

```bash
cargo test --test i18n_test
```

This test programmatically:
1. Parses both `app/src/lib/i18n/en.json` and `app/src/lib/i18n/ar.json`.
2. Verifies that every single nested key in `en.json` exists in `ar.json`.
3. Asserts that no empty translation strings exist.

---

## 4. How to Add a New Language

1. Create `app/src/lib/i18n/<lang_code>.json`.
2. Copy all keys from `en.json` and provide authentic translations.
3. Register the new language code in `app/src/lib/i18n/index.ts`.
4. Run `cargo test --test i18n_test` to verify 100% key parity.
