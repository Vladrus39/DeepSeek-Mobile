#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEFAULT_BINARY="$ROOT/target/release/deepseek-pc-host"
if [[ -x "$ROOT/tools/pc-host/bin/linux-x86_64/deepseek-pc-host" ]]; then
  DEFAULT_BINARY="$ROOT/tools/pc-host/bin/linux-x86_64/deepseek-pc-host"
fi
BINARY="${1:-$DEFAULT_BINARY}"
ENV_FILE="${2:-}"
SERVICE_NAME="${DEEPSEEK_PC_HOST_SERVICE:-deepseek-pc-host}"
UNIT_PATH="/etc/systemd/system/${SERVICE_NAME}.service"

if [[ ! -f "$BINARY" ]]; then
  echo "Build the host first: cargo build -p deepseek-pc-host --release" >&2
  exit 1
fi

ENV_LINE=""
if [[ -n "$ENV_FILE" && -f "$ENV_FILE" ]]; then
  ENV_LINE="EnvironmentFile=${ENV_FILE}"
else
  ENV_LINE="Environment=DEEPSEEK_PC_HOST_BIND=0.0.0.0:8787
Environment=DEEPSEEK_PC_HOST_WORKSPACE=${DEEPSEEK_PC_HOST_WORKSPACE:-$HOME/projects}"
fi

sudo tee "$UNIT_PATH" >/dev/null <<EOF
[Unit]
Description=DeepSeek PC Host
After=network.target

[Service]
Type=simple
ExecStart=${BINARY}
${ENV_LINE}
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable "${SERVICE_NAME}"
sudo systemctl restart "${SERVICE_NAME}"
echo "Installed and started ${SERVICE_NAME}"
