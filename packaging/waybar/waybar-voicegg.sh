#!/usr/bin/env bash
# VoiceGG Waybar Custom Module Status Provider
# Fast, lightweight JSON output provider for Waybar

if ! pgrep -x "voicegg-daemon" > /dev/null 2>&1; then
    echo '{"text":"󰝟 OFF","alt":"offline","tooltip":"VoiceGG daemon is offline. Click to launch.","class":"offline"}'
    exit 0
fi

# Query daemon status via voicegg CLI
STATUS=$(voicegg status --json 2>/dev/null)
if [ -z "$STATUS" ]; then
    echo '{"text":"󰝟 DISCONNECTED","alt":"disconnected","tooltip":"VoiceGG daemon socket disconnected","class":"offline"}'
    exit 0
fi

# Parse status JSON with jq or fallback python
if command -v jq >/dev/null 2>&1; then
    MASTER_VOL=$(echo "$STATUS" | jq -r '.config.volumes.master // 100')
    MASTER_MUTED=$(echo "$STATUS" | jq -r '.config.muted.master // false')
    MIC_MUTED=$(echo "$STATUS" | jq -r '.config.muted.mic // false')
    GAME_NAME=$(echo "$STATUS" | jq -r '.active_game // empty')
else
    # Python fallback if jq is not installed
    read -r MASTER_VOL MASTER_MUTED MIC_MUTED GAME_NAME <<< $(python3 -c "
import sys, json
d = json.loads(sys.argv[1])
v = d.get('config', {}).get('volumes', {}).get('master', 100)
mm = d.get('config', {}).get('muted', {}).get('master', False)
micm = d.get('config', {}).get('muted', {}).get('mic', False)
g = d.get('active_game') or ''
print(f'{v} {mm} {micm} {g}')
" "$STATUS")
fi

# Icons & labels
if [ "$MASTER_MUTED" = "true" ]; then
    VOL_ICON="󰝟"
    VOL_TEXT="MUTED"
else
    VOL_ICON="󰕾"
    VOL_TEXT="${MASTER_VOL}%"
fi

if [ "$MIC_MUTED" = "true" ]; then
    MIC_ICON="󰍭"
else
    MIC_ICON="󰍬"
fi

DISPLAY_TEXT="${VOL_ICON} ${VOL_TEXT} | ${MIC_ICON}"
if [ -n "$GAME_NAME" ]; then
    DISPLAY_TEXT="${DISPLAY_TEXT} [󰊴 ${GAME_NAME}]"
fi

TOOLTIP="VoiceGG — Sonar for Linux\nMaster Volume: ${MASTER_VOL}%\nMaster Muted: ${MASTER_MUTED}\nMic Muted: ${MIC_MUTED}"
if [ -n "$GAME_NAME" ]; then
    TOOLTIP="${TOOLTIP}\nAuto-Detected Game: ${GAME_NAME}"
fi

if command -v jq >/dev/null 2>&1; then
    jq -c -n \
        --arg text "$DISPLAY_TEXT" \
        --arg tooltip "$TOOLTIP" \
        --arg class "active" \
        '{"text": $text, "tooltip": $tooltip, "class": $class}'
else
    python3 -c "
import sys, json
print(json.dumps({'text': sys.argv[1], 'tooltip': sys.argv[2], 'class': 'active'}))
" "$DISPLAY_TEXT" "$TOOLTIP"
fi
