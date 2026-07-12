#!/usr/bin/env bash

BW_CMD="bw"
#BW_CMD="flatpak run --command=bw com.bitwarden.desktop"

session_file="$HOME/.cache/bw_session"
session_time_file="$HOME/.cache/bw_session_time"
max_age=1200 # 20 minutes

clear_session() {
  $BW_CMD lock >/dev/null 2>&1
  rm -f "$session_file" "$session_time_file"
}

if [ -f "$session_file" ] && [ -f "$session_time_file" ]; then
  age=$(($(date +%s) - $(cat "$session_time_file")))
  if [ "$age" -gt "$max_age" ]; then
    clear_session
  fi
fi

items_json=""
if [ -f "$session_file" ]; then
  BW_SESSION=$(cat "$session_file")
  items_json=$($BW_CMD list items --session "$BW_SESSION" </dev/null 2>/dev/null)
  [ -n "$items_json" ] || {
    clear_session
    BW_SESSION=""
  }
fi

if [ -z "$BW_SESSION" ]; then
  echo "🔒 Bitwarden bloqueado, ingresá tu clave maestra:"
  BW_SESSION=$($BW_CMD unlock --raw)
  if [ -z "$BW_SESSION" ]; then
    echo "❌ No se pudo desbloquear Bitwarden."
    read -r -p "Presioná Enter para cerrar..."
    exit 1
  fi
  install -m 600 /dev/null "$session_file"
  echo -n "$BW_SESSION" >"$session_file"
  ts=$(date +%s)
  echo "$ts" >"$session_time_file"

  swaymsg exec "bash -c 'sleep $max_age; [ \"\$(cat \"$session_time_file\" 2>/dev/null)\" = \"$ts\" ] && { $BW_CMD lock >/dev/null 2>&1; rm -f \"$session_file\" \"$session_time_file\"; }'" >/dev/null 2>&1

  items_json=$($BW_CMD list items --session "$BW_SESSION")
fi

export BW_SESSION

if [ -z "$items_json" ] || [ "$items_json" = "[]" ]; then
  echo "⚠️ No se encontraron ítems en Bitwarden."
  read -r -p "Presioná Enter para cerrar..."
  exit 1
fi

selection=$(
  echo "$items_json" |
    jq -r '.[] | select(.name != null) | [.id, .name, (.login.username // "-")] | @tsv' |
    fzf --ansi --height=50% --reverse --prompt="Seleccioná una entrada: " \
      --header="ID\tNOMBRE\tUSUARIO" |
    awk -F'\t' '{print $1}'
)

if [ -z "$selection" ]; then
  echo "❌ No se seleccionó ningún ítem."
  exit 0
fi

password=$($BW_CMD get password "$selection" --session "$BW_SESSION" 2>/dev/null)

if [ -z "$password" ]; then
  echo "⚠️ No se pudo obtener la contraseña para el ítem con ID '$selection'."
  read -r -p "Presioná Enter para cerrar..."
  exit 1
fi

echo -n "$password" | wl-copy
notify-send "✅ Contraseña copiada al portapapeles para el ítem ID: $selection"

deadline=$(($(date +%s) + 9))
deleted=0
while [ "$(date +%s)" -lt "$deadline" ]; do
  if [ "$deleted" -eq 0 ] && cliphist list | grep -qF "$password"; then
    cliphist delete-query "$password" >/dev/null 2>&1
    deleted=1
  fi
  sleep 0.3
done
cliphist delete-query "$password" >/dev/null 2>&1
if [ "$(wl-paste -n 2>/dev/null)" = "$password" ]; then
  wl-copy --clear
  notify-send "🧹 Portapapeles limpiado."
fi
