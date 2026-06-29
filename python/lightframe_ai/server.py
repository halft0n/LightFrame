"""JSON-RPC 2.0 server over stdin/stdout (one request per line)."""

from __future__ import annotations

import json
import sys
import traceback
from typing import Any, Callable

from lightframe_ai import __version__

Handler = Callable[[dict[str, Any]], Any]

_METHODS: dict[str, Handler] = {}
_shutdown_requested = False

_CLIP_AVAILABLE = False
_FACE_AVAILABLE = False
_clip_cache: tuple[Any, Any, Any] | None = None
_face_cache: Any | None = None

try:
    import open_clip  # type: ignore[import-not-found]
    import torch  # type: ignore[import-not-found]

    _CLIP_AVAILABLE = True
except ImportError:
    pass

try:
    import insightface  # type: ignore[import-not-found]

    _FACE_AVAILABLE = True
except ImportError:
    pass


def _register(name: str):
    def decorator(fn: Handler) -> Handler:
        _METHODS[name] = fn
        return fn

    return decorator


def _get_clip_model() -> tuple[Any, Any, Any]:
    """Lazy-load open-clip model, preprocess, and tokenizer."""
    global _clip_cache
    if not _CLIP_AVAILABLE:
        raise RuntimeError("open-clip-torch is not installed")
    if _clip_cache is None:
        model, _, preprocess = open_clip.create_model_and_transforms(  # type: ignore[name-defined]
            "ViT-B-32", pretrained="openai"
        )
        tokenizer = open_clip.get_tokenizer("ViT-B-32")  # type: ignore[name-defined]
        model.eval()
        _clip_cache = (model, preprocess, tokenizer)
    return _clip_cache


def _get_face_analyzer() -> Any:
    """Lazy-load insightface FaceAnalysis model."""
    global _face_cache
    if not _FACE_AVAILABLE:
        raise RuntimeError("insightface is not installed")
    if _face_cache is None:
        app = insightface.app.FaceAnalysis(providers=["CPUExecutionProvider"])  # type: ignore[attr-defined]
        app.prepare(ctx_id=0, det_size=(640, 640))
        _face_cache = app
    return _face_cache


@_register("ping")
def _ping(_params: dict[str, Any]) -> dict[str, bool]:
    return {"pong": True}


@_register("get_version")
def _get_version(_params: dict[str, Any]) -> dict[str, str]:
    return {"version": __version__}


@_register("check_capabilities")
def _check_capabilities(_params: dict[str, Any]) -> dict[str, Any]:
    return {
        "clip": _CLIP_AVAILABLE,
        "face_detection": _FACE_AVAILABLE,
        "onnxruntime": _onnxruntime_available(),
    }


@_register("compute_clip_embedding")
def _compute_clip_embedding(params: dict[str, Any]) -> dict[str, Any] | None:
    """Compute a CLIP image embedding using open-clip when available."""
    image_path = params.get("image_path")
    if not image_path or not isinstance(image_path, str):
        raise ValueError("params.image_path must be a non-empty string")

    if not _CLIP_AVAILABLE:
        return None

    from PIL import Image

    model, preprocess, _ = _get_clip_model()

    with torch.no_grad():  # type: ignore[name-defined]
        image = preprocess(Image.open(image_path).convert("RGB")).unsqueeze(0)
        embedding = model.encode_image(image)
        embedding = embedding / embedding.norm(dim=-1, keepdim=True)
        vector = embedding.squeeze(0).tolist()

    return {"embedding": vector}


@_register("compute_text_embedding")
def _compute_text_embedding(params: dict[str, Any]) -> dict[str, Any] | None:
    """Compute a CLIP text embedding using open-clip when available."""
    text = params.get("text")
    if not text or not isinstance(text, str):
        raise ValueError("params.text must be a non-empty string")

    if not _CLIP_AVAILABLE:
        return None

    model, _, tokenizer = _get_clip_model()

    with torch.no_grad():  # type: ignore[name-defined]
        tokens = tokenizer([text.strip()])
        embedding = model.encode_text(tokens)
        embedding = embedding / embedding.norm(dim=-1, keepdim=True)
        vector = embedding.squeeze(0).tolist()

    return {"embedding": vector}


@_register("detect_faces")
def _detect_faces(params: dict[str, Any]) -> dict[str, Any]:
    """Detect faces using insightface when available."""
    image_path = params.get("image_path")
    if not image_path or not isinstance(image_path, str):
        raise ValueError("params.image_path must be a non-empty string")

    if not _FACE_AVAILABLE:
        return {"faces": []}

    app = _get_face_analyzer()
    detections = app.get(image_path)

    faces = []
    for face in detections:
        bbox = face.bbox.astype(float).tolist()
        embedding = face.embedding.astype(float).tolist() if face.embedding is not None else []
        faces.append(
            {
                "bbox": bbox,
                "confidence": float(face.det_score),
                "embedding": embedding,
            }
        )

    return {"faces": faces}


@_register("classify_screenshot")
def _classify_screenshot(params: dict[str, Any]) -> dict[str, Any]:
    """Classify a screenshot image (stub — returns confidence placeholder)."""
    path = params.get("path")
    if not path or not isinstance(path, str):
        raise ValueError("params.path must be a non-empty string")

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


def _onnxruntime_available() -> bool:
    try:
        import onnxruntime  # type: ignore[import-not-found]  # noqa: F401

        return True
    except ImportError:
        return False


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
