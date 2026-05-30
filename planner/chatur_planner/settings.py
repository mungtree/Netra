"""Env-driven sidecar configuration."""

from __future__ import annotations

import os
from dataclasses import dataclass


@dataclass(frozen=True)
class Settings:
    port: int
    llamacpp_base_url: str
    model: str
    api_key: str

    @classmethod
    def from_env(cls) -> "Settings":
        return cls(
            port=int(os.environ.get("CHATUR_PLANNER_PORT", "8899")),
            llamacpp_base_url=os.environ.get(
                "CHATUR_PLANNER_LLAMACPP_URL", "http://127.0.0.1:8888/v1"
            ),
            model=os.environ.get("CHATUR_PLANNER_MODEL", "qwen3.6-35b-a3b"),
            api_key=os.environ.get("CHATUR_PLANNER_API_KEY", "not-needed"),
        )
