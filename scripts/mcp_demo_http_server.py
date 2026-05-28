#!/usr/bin/env python3
"""
Minimal MCP JSON-RPC server over HTTP POST.

Implements:
  - initialize
  - tools/list
  - tools/call (echo)

This is intentionally dependency-free for quick on-device E2E.
"""

from __future__ import annotations

import argparse
import json
from http.server import BaseHTTPRequestHandler, HTTPServer
from typing import Any, Dict


def jsonrpc_result(id_value: Any, result: Any) -> bytes:
    return json.dumps({"jsonrpc": "2.0", "id": id_value, "result": result}).encode("utf-8")


def jsonrpc_error(id_value: Any, message: str) -> bytes:
    return json.dumps(
        {"jsonrpc": "2.0", "id": id_value, "error": {"message": message}}
    ).encode("utf-8")


TOOLS = [
    {
        "name": "echo",
        "description": "Echo back provided text.",
        "inputSchema": {
            "type": "object",
            "properties": {"text": {"type": "string"}},
            "required": ["text"],
            "additionalProperties": False,
        },
    }
]


class Handler(BaseHTTPRequestHandler):
    def do_POST(self) -> None:
        if self.path.rstrip("/") != "/mcp":
            self.send_response(404)
            self.end_headers()
            return

        length = int(self.headers.get("Content-Length", "0"))
        raw = self.rfile.read(length) if length > 0 else b"{}"
        try:
            payload: Dict[str, Any] = json.loads(raw.decode("utf-8"))
        except Exception:
            self.send_response(400)
            self.end_headers()
            self.wfile.write(b"invalid json")
            return

        rid = payload.get("id")
        method = payload.get("method")
        params = payload.get("params") or {}

        if method == "initialize":
            out = jsonrpc_result(
                rid,
                {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "serverInfo": {"name": "mcp-demo-http", "version": "0.0.1"},
                },
            )
        elif method == "tools/list":
            out = jsonrpc_result(rid, {"tools": TOOLS})
        elif method == "tools/call":
            name = params.get("name")
            arguments = params.get("arguments") or {}
            if name != "echo":
                out = jsonrpc_error(rid, f"unknown tool: {name}")
            else:
                text = arguments.get("text", "")
                out = jsonrpc_result(
                    rid,
                    {
                        "content": [
                            {"type": "text", "text": f"ECHO: {text}"},
                        ]
                    },
                )
        else:
            out = jsonrpc_error(rid, f"unknown method: {method}")

        self.send_response(200)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(out)))
        self.end_headers()
        self.wfile.write(out)

    def log_message(self, format: str, *args: Any) -> None:
        # Quiet by default (avoid noisy terminal logs).
        return


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", default="0.0.0.0")
    ap.add_argument("--port", type=int, default=3333)
    args = ap.parse_args()
    server = HTTPServer((args.host, args.port), Handler)
    server.serve_forever()


if __name__ == "__main__":
    main()

