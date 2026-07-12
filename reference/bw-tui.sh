#!/usr/bin/env bash

config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/bw-tui"
config_file="$config_dir/config.json"

if [ ! -f "$config_file" ]; then
  mkdir -p "$config_dir"
  cat >"$config_file" <<'EOF'
{
  "bw_cmd": "bw",
  "session_max_age_secs": 1200,
  "clipboard_clear_secs": 9,
  "generator": {
    "length": 20,
    "uppercase": true,
    "lowercase": true,
    "numbers": true,
    "special": false
  }
}
EOF
fi

BW_CMD=$(jq -r '.bw_cmd // "bw"' "$config_file")
max_age=$(jq -r '.session_max_age_secs // 1200' "$config_file")
clear_secs=$(jq -r '.clipboard_clear_secs // 9' "$config_file")

cache_dir="${XDG_CACHE_HOME:-$HOME/.cache}/bw-tui"
session_file="$cache_dir/session"
session_time_file="$cache_dir/session_time"

clear_session() {
  $BW_CMD lock >/dev/null 2>&1
  rm -f "$session_file" "$session_time_file"
}

copy_and_autoclear() {
  local value="$1" label="$2"
  echo -n "$value" | wl-copy
  notify-send "✅ $label copiado al portapapeles."

  local deadline=$(($(date +%s) + clear_secs))
  local deleted=0
  while [ "$(date +%s)" -lt "$deadline" ]; do
    if [ "$deleted" -eq 0 ] && cliphist list | grep -qF "$value"; then
      cliphist delete-query "$value" >/dev/null 2>&1
      deleted=1
    fi
    sleep 0.3
  done
  cliphist delete-query "$value" >/dev/null 2>&1
  if [ "$(wl-paste -n 2>/dev/null)" = "$value" ]; then
    wl-copy --clear
    notify-send "🧹 Portapapeles limpiado."
  fi
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
  mkdir -p "$cache_dir"
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
    jq -r '.[] | select(.name != null) |
      [.id,
       (if .type==1 then "Login" elif .type==2 then "Nota" elif .type==3 then "Tarjeta" elif .type==4 then "Identidad" else "?" end),
       .name,
       (.login.username // "-")] | @tsv' |
    fzf --ansi --height=50% --reverse --prompt="Seleccioná una entrada: " \
      --header="ID\tTIPO\tNOMBRE\tUSUARIO" |
    awk -F'\t' '{print $1}'
)

if [ -z "$selection" ]; then
  echo "❌ No se seleccionó ningún ítem."
  exit 0
fi

item_type=$(echo "$items_json" | jq -r --arg id "$selection" '.[] | select(.id==$id) | .type')

case "$item_type" in
1)
  password=$($BW_CMD get password "$selection" --session "$BW_SESSION" 2>/dev/null)
  if [ -z "$password" ]; then
    echo "⚠️ No se pudo obtener la contraseña para el ítem con ID '$selection'."
    read -r -p "Presioná Enter para cerrar..."
    exit 1
  fi
  copy_and_autoclear "$password" "Contraseña"
  ;;
2)
  notes=$(echo "$items_json" | jq -r --arg id "$selection" '.[] | select(.id==$id) | .notes // empty')
  if [ -z "$notes" ]; then
    echo "⚠️ Esa nota no tiene contenido."
    read -r -p "Presioná Enter para cerrar..."
    exit 1
  fi
  copy_and_autoclear "$notes" "Nota"
  ;;
3)
  field=$(printf 'Número\nCVV' | fzf --height=20% --reverse --prompt="Copiar: ")
  case "$field" in
  Número)
    value=$(echo "$items_json" | jq -r --arg id "$selection" '.[] | select(.id==$id) | .card.number // empty')
    label="Número de tarjeta"
    ;;
  CVV)
    value=$(echo "$items_json" | jq -r --arg id "$selection" '.[] | select(.id==$id) | .card.code // empty')
    label="CVV"
    ;;
  *)
    echo "❌ No se seleccionó ningún campo."
    exit 0
    ;;
  esac
  if [ -z "$value" ]; then
    echo "⚠️ Esa tarjeta no tiene ese dato cargado."
    read -r -p "Presioná Enter para cerrar..."
    exit 1
  fi
  copy_and_autoclear "$value" "$label"
  ;;
*)
  echo "⚠️ Tipo de ítem no soportado todavía."
  read -r -p "Presioná Enter para cerrar..."
  exit 1
  ;;
esac
