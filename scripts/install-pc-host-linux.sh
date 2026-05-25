#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY="${1:-$ROOT/target/release/deepseek-pc-host}"
SERVICE_NAME="${DEEPSEEK_PC_HOST_SERVICE:-deepseek-pc-host}"
UNIT_PATH="/etc/systemd/system/${SERVICE_NAME}.service"

if [[ ! -f "$BINARY" ]]; then
  echo "Build the host first: cargo build -p deepseek-pc-host --release" >&2
  exit 1
fi

sudo tee "$UNIT_PATH" >/dev/null <<EOF
[Unit]
Description=DeepSeek PC Host
After=network.target

[Service]
Type=simple
ExecStart=${BINARY}
Environment=DEEPSEEK_PC_HOST_BIND=0.0.0.0:8787
Environment=DEEPSEEK_PC_HOST_WORKSPACE=${DEEPSEEK_PC_HOST_WORKSPACE:-$HOME/projects}
Restart=on-failure

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable "${SERVICE_NAME}"
sudo systemctl restart "${SERVICE_NAME}"
echo "Installed and started ${SERVICE_NAME}"
