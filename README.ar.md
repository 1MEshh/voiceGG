# VoiceGG — البديل الحقيقي لبرنامج SteelSeries Sonar على لينكس

<p align="center">
  <img src="packaging/icons/voicegg.svg" alt="VoiceGG Logo" width="120" height="120" />
</p>

<p align="center">
  <b>موجّه الصوت المخصص للألعاب، ومعادل الترددات البارامتري ذو الـ 10 نطاقات، ووحدة التحكم في موازنة الصوت ChatMix لنظام لينكس و PipeWire.</b>
  <br />
  <i>البديل الأصلي لبرنامج SteelSeries Sonar، تم تطويره بلغة Rust لنظام Arch Linux ومديري النوافذ Hyprland و Sway وبيئات KDE Plasma و GNOME.</i>
</p>

<p align="center">
  <a href="https://github.com/1MEshh/voiceGG/releases/tag/v2.0.0"><img src="https://img.shields.io/badge/الإصدار-v2.0.0%20(Sonar%20Edition)-FF5200?style=for-the-badge&logo=rocket" alt="الإصدار 2.0" /></a>
  <a href="https://github.com/1MEshh/voiceGG"><img src="https://img.shields.io/badge/SteelSeries%20Sonar-بديل%20لينكس-FF5200?style=for-the-badge&logo=linux&logoColor=white" alt="بديل سونار" /></a>
  <img src="https://img.shields.io/badge/اللغة-Rust%202021-orange?style=for-the-badge&logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/خادم%20الصوت-PipeWire%20Native-blue?style=for-the-badge&logo=buffer" alt="PipeWire" />
  <img src="https://img.shields.io/badge/الواجهة-Tauri%20v2%20%2B%20Svelte%205-FF3E00?style=for-the-badge&logo=svelte" alt="Tauri + Svelte" />
  <img src="https://img.shields.io/badge/استهلاك%20الذاكرة-~6.1%20ميغابايت-success?style=for-the-badge" alt="6MB RAM" />
  <img src="https://img.shields.io/badge/التتبع-0%25%20(يعمل%20بدون%20إنترنت)-brightgreen?style=for-the-badge" alt="خصوصية تامة" />
  <img src="https://img.shields.io/badge/الترخيص-GPLv3-green?style=for-the-badge" alt="GPLv3" />
</p>

<p align="center">
  <b>اللغة:</b> <a href="README.md">English (الإنجليزية)</a> | <a href="README.ar.md">العربية</a>
</p>

---

## ⚡ التثبيت السريع (Arch Linux وأي توزيعة تدعم PipeWire)

شغّل هذا الأمر في الطرفية (Terminal) لتثبيت VoiceGG وبنائه لجهازك دون الحاجة لصلاحيات root:

```bash
curl -fsSL https://raw.githubusercontent.com/1MEshh/voiceGG/main/install.sh | bash
```

> [!TIP]
> يمكنك تشغيل التطبيق في أي وقت من مشغل التطبيقات المفضل لديك (**Rofi**, **Wofi**, **tofi**, **dmenu**) أو عبر كتابة `voicegg-launcher` في الطرفية.

---

## 📖 قصة ولادة VoiceGG

عندما مسحت نظام ويندوز وقررت الانتقال كلياً إلى نظام **Arch Linux** مع مدير النوافذ **Hyprland** كبيئتي الأساسية للألعاب والعمل، كان هناك برنامج احتكاري واحد على ويندوز افتقدته بشدة: **SteelSeries Sonar (SteelSeries GG)**.

لسنوات طويلة على ويندوز، كانت تجربة الألعاب التنافسية لدي مبنية بالكامل حول Sonar:
1. وجود قنوات افتراضية منفصلة لـ **الألعاب (Game)** و **المحادثات (Chat)** و **الوسائط (Media)**، مما يتيح لي موازنة صوت الديسكورد وصوت اللعبة في أجزاء من الثانية.
2. ميزة **ChatMix** التي تمكّنني فوراً من خفض صوت الزملاء المزعجين أثناء الجولات الحاسمة في ألعاب مثل *Counter-Strike* و *Apex Legends*.
3. **معادل صوت بارامتري من 10 نطاقات (10-Band EQ)** مع ملفات ضبط مخصصة لعزل أصوات خطوات الأقدام بدقة فائقة.
4. **مراقبة صوت الميكروفون المباشرة (Live Sidetone)** حتى أسمع صوتي الطبيعي ولا أضطر للصراخ في سماعة الرأس أثناء التحدث مع أصدقائي.

عندما بحثت عن بديل حقيقي على لينكس، اصطدمت بحائط مسدود:
- ❌ **لوحات التوصيل التقليدية (qpwgraph, Helvum)**: شبكات معقدة ومزعجة من مئات الخطوط والوصلات الافتراضية، بدون أي إعدادات مسبقة للألعاب، وبدون حفظ تلقائي لتوجيه التطبيقات، وبدون ميزة ChatMix.
- ❌ **تشغيل SteelSeries GG عبر Wine/Proton**: مستحيل تماماً لأنه يعتمد على تعريفات صوت منخفضة المستوى لنواة ويندوز، ويستهلك في الخلفية ما بين **800 إلى 1200 ميجابايت من الذاكرة العشوائية (RAM)**، ويشغّل خدمات تتبع وإعلانات لا تتوقف.
- ❌ **إعدادات الصوت العادية**: تفتقر إلى ذكاء توجيه البرامج ومعالجة الإشارات الصوتية والاندماج مع شريط Waybar و Hyprland.

**رفضت الاستسلام لهذه المشكلة، فقررت بناء VoiceGG بنفسي.**

تم بناء VoiceGG من الصفر بلغة **Rust** السريعة والآمنة، ليعمل مباشرة فوق خادم الصوت الحديث **PipeWire**، ليمنح مجتمع لاعبي لينكس تجربة سونار المتكاملة ولكن بمواصفات تقنية متفوقة:
- يستهلك فقط **~6 ميجابايت من الذاكرة (RAM)** (أخف بأكثر من **100 ضعف** من برنامج SteelSeries الأصلي على ويندوز).
- يستهلك **0.0% من المعالج (CPU)** في وضع الخمول، مع زمن استجابة ومعالجة صوتية فائق السرعة يقل عن 6 أجزاء من الألف من الثانية.
- **بدون أي تتبع سحابي (Zero Telemetry)**، وبدون حسابات أو تسجيل دخول، ويعمل محلياً بالكامل ومفتوح المصدر بنسبة 100%.
- يدعم اللغة العربية بالكامل وبشكل أصيل باتجاه **يمين-إلى-يسار (RTL)** مع خط **ثمانية (`ثمانية`)** وخط **IBM Plex Sans Arabic** المتناسق مع عالم الصوتيات الاحترافي.

— صُمم وطُوّر بشغف لمجتمع لاعبي لينكس بواسطة [@1MEshh](https://github.com/1MEshh)

---

## استعراض الواجهة والميزات

### 1. مكساج القنوات المتعددة وميزة ChatMix
قنوات افتراضية مستقلة لكل من: **الرئيسي (Master)**، **الألعاب (Game)**، **المحادثات (Chat)**، **الوسائط (Media)**، **المساعد (Aux)**، و **الميكروفون (Mic)**.
يمكنك سحب وإفلات أي تطبيق يعمل في نظامك (مثل Discord، Steam، Spotify، متصفح الإنترنت) بين القنوات ليتم توجيهه فوراً وحفظ مكانه بشكل دائم. بالإضافة إلى شريط موازنة **ChatMix** الفوري.

---

### 2. معادل الترددات البارامتري (10-Band EQ) وضبط الألعاب
مخطط تفاعلي دقيق لمعادلة الترددات مع إمكانية سحب النقاط وضبط عامل الجودة (Q) وعجلة الفأرة.
يحتوي على إعدادات مسبقة احترافية لأشهر الألعاب التنافسية:
- **Counter-Strike 2**: عزل صوت خطوات الأقدام على الأسطح المعدنية والخشبية، وإلغاء رنين القنابل اليدوية.
- **Apex Legends**: إبراز صوت الانزلاق وخطوات الأعداء عبر الجدران والأسطح.
- **Valorant**: تحسين الصوت المحيطي وتوضيح صوت زرع وإبطال القنبلة (Spike).
- **Escape from Tarkov**: إبراز حفيف الأشجار وصرير الأرضيات الخشبية مع عزل صوت الرياح والأمطار.
- **Rainbow Six Siege**: توضيح صوت الأسلاك الشائكة وحبال الهبوط واختراق الجدران.

---

### 3. معالجة صوت الميكروفون المتقدمة والمراقبة الحية
مجموعة متكاملة من فلاتر معالجة الصوت الاحترافية تعمل داخل محرك PipeWire:
- **مراقبة الصوت المباشرة (Live Monitoring)**: سماع صوتك في الوقت الحقيقي بدون أي تأخير، مع حماية برمجية تمنع حدوث ارتداد الصوت والصفير المزعج (Feedback Guard).
- **مزيل الضوضاء بالذكاء الاصطناعي (RNNoise)**: عزل صوت مراوح التبريد ونقرات لوحة المفاتيح الميكانيكية بذكاء ودون تشويه نبرة صوتك.
- **بوابة الضوضاء (Noise Gate)**: كتم الميكروفون تلقائياً عند التوقف عن الكلام، مع مؤشر بصري حي لحالة البوابة (مفتوحة / مغلقة).
- **الضاغط الصوتي (Compressor)**: موازنة نبرة الصوت وتثبيت مستوى الارتفاع حتى لا تنزعج آذان زملائك عند الانفعال أو الصراخ أثناء اللعب.

---

## مقارنة تقنية مباشرة

| الميزة | VoiceGG (لينكس) | SteelSeries GG (ويندوز) | برامج تحكم لينكس العامة |
|:---|:---:|:---:|:---:|
| **نظام التشغيل المدعوم** | **لينكس (Arch, Hyprland, Sway, KDE, GNOME)** | ويندوز 10/11 فقط | لينكس |
| **استهلاك الذاكرة في الخمول** | **~6.1 ميجابايت** | ~840 إلى 1200 ميجابايت | ~120 إلى 250 ميجابايت |
| **استهلاك المعالج في الخمول** | **0.00%** | 1.8% – 4.5% | 0.8% – 1.5% |
| **التتبع وجمع البيانات** | **صفر (0%) — خصوصية محلية تامة** | خدمات تتبع وإعلانات مستمرة | لا يوجد |
| **قنوات صوتية مخصصة للألعاب** | **نعم (Master, Game, Chat, Media, Aux, Mic)** | نعم (5 قنوات) | تتطلب ربط يدوي معقد |
| **سحب وإفلات البرامج وحفظها** | **نعم (فوري ودائم)** | نعم | لا |
| **ميزة ChatMix بمفاتيح اختصار** | **نعم (اختصارات، واجهة، و Waybar)** | نعم (عتاد SteelSeries أو واجهة) | لا |
| **معادل صوت 10-Band للألعاب** | **نعم (تفاعلي مع ملفات ألعاب جاهزة)** | نعم | تتطلب إضافات طرف ثالث |
| **سماع صوت الميكروفون المباشر** | **نعم (مع حماية فورية من الارتداد)** | نعم | يدوي ومعقد |
| **دعم اللغة العربية والخطوط الأصيلة** | **نعم (RTL كامل مع خط ثمانية)** | دعم جزئي ضعيف | لا |
| **إعادة ضبط الصوت الطارئة (Panic)** | **نعم (زر واحد لإرجاع الصوت للعتاد الأصلي)** | إعادة تثبيت التعريفات | إعادة تشغيل PipeWire |

---

## التكامل مع شريط Waybar واختصارات Hyprland

### شريط Waybar
أضف الوحدة إلى ملف إعدادات شريطك `~/.config/waybar/config.jsonc`:

```jsonc
"custom/voicegg": {
    "format": "{}",
    "exec": "$HOME/.local/bin/waybar-voicegg.sh",
    "return-type": "json",
    "interval": 2,
    "on-click": "voicegg-launcher",
    "on-click-middle": "voicegg ctl mute master toggle",
    "on-click-right": "voicegg ctl mute mic toggle",
    "on-scroll-up": "voicegg ctl chatmix +5",
    "on-scroll-down": "voicegg ctl chatmix -5",
    "tooltip": true
}
```

### اختصارات Hyprland (`~/.config/hypr/hyprland.conf`)
```ini
# كتم وتشغيل الميكروفون
bind = SUPER, F9, exec, voicegg ctl mute mic toggle

# تعديل موازنة ChatMix (صوت اللعبة مقابل صوت الديسكورد)
bind = SUPER, F10, exec, voicegg ctl chatmix -10
bind = SUPER, F11, exec, voicegg ctl chatmix +10

# التحكم بالصوت العام
bind = SUPER, F12, exec, voicegg ctl volume master +5
bind = SUPER SHIFT, F12, exec, voicegg ctl volume master -5

# إعادة ضبط الصوت الطارئة في حال حدوث أي خلل
bind = SUPER CTRL, ESCAPE, exec, voicegg panic
```

---

## الأسئلة الشائعة (FAQ)

### هل أحتاج لسماعة من شركة SteelSeries لاستخدام البرنامج وميزة ChatMix؟
**لا على الإطلاق.** برنامج VoiceGG يعمل مع **أي سماعة** أو كرت صوت أو واجهة صوتية (HyperX, Logitech, Sennheiser, Beyerdynamic, Corsair, Razer, Apple EarPods وغيرها)، لأن المعالجة تتم برمجياً داخل خادم PipeWire.

### ما التوزيعات المدعومة؟
البرنامج مطوّر بالأساس وموجّه لـ **Arch Linux** والتوزيعات المبنية عليها (EndeavourOS, Manjaro, Garuda, CachyOS)، ويعمل بسلاسة على أي توزيعة لينكس حديثة تستخدم خادم **PipeWire** (مثل Fedora و Ubuntu 24.04+ و Debian 13+ و openSUSE).

### كيف أحذف البرنامج بشكل كامل وآمن؟
البرنامج يحتوي على سكريبت حذف آمن ينظف كل الملفات دون المساس بملفات نظامك:
```bash
voicegg-uninstall
```
أو عبر الأمر:
```bash
curl -fsSL https://raw.githubusercontent.com/1MEshh/voiceGG/main/uninstall.sh | bash
```

---

## الترخيص والمساهمة

برنامج VoiceGG مجاني ومفتوح المصدر تحت رخصة **GNU General Public License v3.0 (GPLv3)**. جميع المساهمات، تقارير الأخطاء، وملفات ضبط الألعاب مرحب بها بكل سرور!

- **مستودع المشروع على GitHub**: [https://github.com/1MEshh/voiceGG](https://github.com/1MEshh/voiceGG)
- **المطور**: [@1MEshh](https://github.com/1MEshh)
