"""JSON-RPC 2.0 server over stdin/stdout (one request per line)."""

from __future__ import annotations

import json
import sys
import traceback
from typing import Any, Callable

from catchlight_ai import __version__

Handler = Callable[[dict[str, Any]], Any]

_METHODS: dict[str, Handler] = {}
_shutdown_requested = False


def _register(name: str):
    def decorator(fn: Handler) -> Handler:
        _METHODS[name] = fn
        return fn

    return decorator


@_register("ping")
def _ping(_params: dict[str, Any]) -> dict[str, bool]:
    return {"pong": True}


@_register("get_version")
def _get_version(_params: dict[str, Any]) -> dict[str, str]:
    return {"version": __version__}


@_register("classify_screenshot")
def _classify_screenshot(params: dict[str, Any]) -> dict[str, Any]:
    """Classify a screenshot image (stub — returns confidence placeholder)."""
    path = params.get("path")
    if not path or not isinstance(path, str):
        raise ValueError("params.path must be a non-empty string")

    # Placeholder until CLIP/ONNX models are wired up.
    return {
        "path": path,
        "confidence": 0.5,
        "label": "screenshot",
    }


@_register("shutdown")
def _shutdown(_params: dict[str, Any]) -> dict[str, bool]:
    global _shutdown_requested
    _shutdown_requested = True
    return {"ok": True}


def _write_response(response: dict[str, Any]) -> None:
    sys.stdout.write(json.dumps(response, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def _error_response(request_id: Any, code: int, message: str) -> dict[str, Any]:
    return {
        "jsonrpc": "2.0",
        "id": request_id,
        "error": {"code": code, "message": message},
    }


def _success_response(request_id: Any, result: Any) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": request_id, "result": result}


def _handle_request(raw: str) -> dict[str, Any] | None:
    request_id: Any = None
    try:
        request = json.loads(raw)
    except json.JSONDecodeError as exc:
        return _error_response(None, -32700, f"Parse error: {exc}")

    if not isinstance(request, dict):
        return _error_response(None, -32600, "Invalid Request: expected object")

    request_id = request.get("id")
    method = request.get("method")
    params = request.get("params") or {}

    if request.get("jsonrpc") != "2.0":
        return _error_response(request_id, -32600, "Invalid Request: jsonrpc must be '2.0'")

    if not isinstance(method, str):
        return _error_response(request_id, -32600, "Invalid Request: method must be a string")

    if not isinstance(params, dict):
        return _error_response(request_id, -32600, "Invalid Request: params must be an object")

    handler = _METHODS.get(method)
    if handler is None:
        return _error_response(request_id, -32601, f"Method not found: {method}")

    try:
        result = handler(params)
        return _success_response(request_id, result)
    except ValueError as exc:
        return _error_response(request_id, -32602, str(exc))
    except Exception as exc:  # noqa: BLE001 — surface internal errors to Rust caller
        tb = traceback.format_exc()
        sys.stderr.write(f"sidecar error in {method}: {exc}\n{tb}\n")
        sys.stderr.flush()
        return _error_response(request_id, -32603, f"Internal error: {exc}")


def run_server() -> None:
    """Read JSON-RPC requests from stdin until shutdown or EOF."""
    global _shutdown_requested

    for line in sys.stdin:
        stripped = line.strip()
        if not stripped:
            continue

        response = _handle_request(stripped)
        if response is not None:
            _write_response(response)

        if _shutdown_requested:
            break
