#!/usr/bin/env bash
set -euo pipefail

BUNDLE_DIR="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"
SERVICE_NAME="${DEEPSEEK_PC_HOST_SERVICE:-deepseek-pc-host}"
UNIT_PATH="/etc/systemd/system/${SERVICE_NAME}.service"
ENV_FILE="${BUNDLE_DIR}/deepseek-pc-host.env"

if [[ ! -f "$ENV_FILE" ]]; then
  echo "Missing deepseek-pc-host.env in ${BUNDLE_DIR}" >&2
  exit 1
fi

set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a

BINARY=""
for candidate in \
  "${BUNDLE_DIR}/deepseek-pc-host" \
  "${BUNDLE_DIR}/bin/deepseek-pc-host"; do
  if [[ -x "$candidate" ]]; then
    BINARY="$candidate"
    break
  fi
done

if [[ -z "$BINARY" ]]; then
  echo "deepseek-pc-host binary not found in bundle directory" >&2
  exit 1
fi

sudo tee "$UNIT_PATH" >/dev/null <<EOF
[Unit]
Description=DeepSeek PC Host (pairing bundle)
After=network.target

[Service]
Type=simple
ExecStart=${BINARY}
EnvironmentFile=${ENV_FILE}
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable "${SERVICE_NAME}"
sudo systemctl restart "${SERVICE_NAME}"
echo "Installed ${SERVICE_NAME} from pairing bundle (${BINARY})"
