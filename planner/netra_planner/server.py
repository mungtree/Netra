"""FastAPI sidecar producing schema-constrained JSON.

Backend: existing llama.cpp OpenAI-compatible endpoint. Uses
`response_format={"type":"json_schema", ...}` which llama.cpp's server
enforces with its native grammar engine. `outlines.from_openai` wraps the
exact same call when available — we fall back to a raw OpenAI SDK request
so the sidecar works regardless of the installed `outlines` version.
"""

from __future__ import annotations

import json
import logging
import time
from typing import Any

from fastapi import FastAPI, HTTPException
from openai import OpenAI
from pydantic import BaseModel, Field

from .settings import Settings


logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(name)s: %(message)s")
log = logging.getLogger("netra_planner")
settings = Settings.from_env()
app = FastAPI(title="netra-planner", version="0.1.0")


class GenerateRequest(BaseModel):
    prompt: str
    schema_: dict[str, Any] = Field(alias="schema")
    max_tokens: int = 16000
    temperature: float = 0.3
    model: str | None = None
    base_url: str | None = None

    class Config:
        populate_by_name = True


def _client(base_url: str) -> OpenAI:
    return OpenAI(base_url=base_url, api_key=settings.api_key)


@app.get("/healthz")
async def healthz() -> dict[str, Any]:
    return {
        "ok": True,
        "model": settings.model,
        "base_url": settings.llamacpp_base_url,
    }


@app.post("/generate")
async def generate(req: GenerateRequest) -> dict[str, Any]:
    model_id = req.model or settings.model
    base_url = req.base_url or settings.llamacpp_base_url
    schema = {k: v for k, v in req.schema_.items() if k != "$schema"}

    # JSON-Schema response format: llama.cpp enforces this server-side with its
    # GBNF grammar engine.
    response_format = {
        "type": "json_schema",
        "json_schema": {
            "name": schema.get("title", "Response"),
            "schema": schema,
        },
    }

    log.info(
        "generate: model=%s base_url=%s prompt_len=%d schema_keys=%s",
        model_id, base_url, len(req.prompt), list(schema.keys()),
    )
    started = time.monotonic()
    try:
        client = _client(base_url)
        completion = client.chat.completions.create(
            model=model_id,
            messages=[{"role": "user", "content": req.prompt}],
            max_tokens=req.max_tokens,
            temperature=req.temperature,
            response_format=response_format,
        )
    except Exception as exc:
        log.exception("generation request failed")
        raise HTTPException(
            status_code=502, detail=f"generation failed: {exc}"
        ) from exc

    if not completion.choices:
        raise HTTPException(status_code=502, detail="empty completion")
    content = completion.choices[0].message.content or ""
    try:
        value = json.loads(content)
    except json.JSONDecodeError as exc:
        raise HTTPException(
            status_code=502,
            detail=f"model returned non-JSON despite response_format: {exc}: {content[:200]}",
        ) from exc

    elapsed_ms = int((time.monotonic() - started) * 1000)
    log.info("generate: ok elapsed_ms=%d content_len=%d", elapsed_ms, len(content))
    return {"value": value}


def main() -> None:
    import uvicorn

    uvicorn.run(
        "netra_planner.server:app",
        host="127.0.0.1",
        port=settings.port,
        log_level="info",
    )


if __name__ == "__main__":
    main()
