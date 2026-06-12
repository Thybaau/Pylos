from __future__ import annotations

import logging
from functools import lru_cache

from mem0 import Memory

from pylos.memory.config import Mem0Settings
from pylos.memory.models import MemoryResult, UserProfile

log = logging.getLogger(__name__)


class PylosMemoryManager:
    def __init__(self, config: Mem0Settings | None = None) -> None:
        self._config = config or Mem0Settings.from_env()
        self._memory: Memory = Memory.from_config(self._config.mem0_config)
        self.max_context_tokens: int = self._config.max_context_tokens
        self.ttl_default: int = self._config.ttl_seconds
        self.search_limit: int = self._config.search_limit
        log.info(
            "PylosMemoryManager initialized (backend=%s, collection=%s, max_tokens=%d)",
            self._config.backend,
            self._config.collection_name,
            self.max_context_tokens,
        )

    @property
    def memory(self) -> Memory:
        return self._memory

    @property
    def config(self) -> Mem0Settings:
        return self._config

    def add_user_memory(
        self,
        user_id: str,
        data: str,
        metadata: dict | None = None,
    ) -> str:
        meta = {"type": "user_memory", "source": "interaction"}
        if metadata:
            meta.update(metadata)
        result = self._memory.add(
            data,
            user_id=user_id,
            metadata=meta,
        )
        memory_id = self._extract_id(result)
        log.debug("Stored user memory %s for user %s", memory_id, user_id)
        return memory_id

    def add_session_memory(
        self,
        session_id: str,
        user_id: str,
        data: str,
        metadata: dict | None = None,
    ) -> str:
        meta = {"type": "session_memory", "source": "interaction"}
        if metadata:
            meta.update(metadata)
        result = self._memory.add(
            data,
            user_id=user_id,
            session_id=session_id,
            metadata=meta,
        )
        memory_id = self._extract_id(result)
        log.debug("Stored session memory %s for session %s", memory_id, session_id)
        return memory_id

    def search(
        self,
        query: str,
        user_id: str | None = None,
        session_id: str | None = None,
        limit: int | None = None,
    ) -> list[MemoryResult]:
        results = self._memory.search(
            query,
            user_id=user_id,
            session_id=session_id,
            limit=limit or self.search_limit,
        )
        return [self._to_result(r) for r in results]

    def get_relevant_context(
        self,
        query: str,
        user_id: str | None = None,
        session_id: str | None = None,
        max_tokens: int | None = None,
    ) -> tuple[str, int, int]:
        budget = max_tokens or self.max_context_tokens
        all_results = self.search(query, user_id=user_id, session_id=session_id)
        if not all_results:
            return "", 0, 0
        filtered = self._filter_by_token_budget(all_results, budget)
        context = self._format_context(user_id, session_id, filtered)
        token_estimate = _estimate_tokens(context)
        return context, len(filtered), token_estimate

    def update_memory(
        self,
        memory_id: str,
        data: str | None = None,
        metadata: dict | None = None,
    ) -> bool:
        try:
            self._memory.update(memory_id, data=data, metadata=metadata)
            log.debug("Updated memory %s", memory_id)
            return True
        except Exception:
            log.exception("Failed to update memory %s", memory_id)
            return False

    def delete_memory(self, memory_id: str) -> bool:
        try:
            self._memory.delete(memory_id)
            log.debug("Deleted memory %s", memory_id)
            return True
        except Exception:
            log.exception("Failed to delete memory %s", memory_id)
            return False

    def get_user_profile(self, user_id: str) -> UserProfile:
        results = self._memory.get_all(user_id=user_id)
        if not results:
            return UserProfile(user_id=user_id)
        preferences: list[str] = []
        habits: list[str] = []
        facts: list[str] = []
        for entry in results:
            text = (entry.get("text") or entry.get("memory", "")).strip()
            if not text:
                continue
            meta = entry.get("metadata", {}) or {}
            entry_type = meta.get("type", "")
            if "prefer" in text.lower():
                preferences.append(text)
            elif entry_type == "user_memory" or "habit" in text.lower():
                habits.append(text)
            else:
                facts.append(text)
        return UserProfile(
            user_id=user_id,
            preferences=preferences,
            habits=habits,
            facts=facts,
            memory_count=len(results),
            last_active=None,
        )

    def _filter_by_token_budget(
        self,
        results: list[MemoryResult],
        max_tokens: int,
    ) -> list[MemoryResult]:
        filtered: list[MemoryResult] = []
        total = 0
        overhead = _estimate_tokens(
            "## User Memory\n## Session Memory\n"
        )
        remaining = max_tokens - overhead
        for r in sorted(results, key=lambda x: x.score or 0.0, reverse=True):
            tokens = _estimate_tokens(f"- {r.text}\n")
            if total + tokens > remaining:
                break
            filtered.append(r)
            total += tokens
        return filtered

    def _format_context(
        self,
        user_id: str | None,
        session_id: str | None,
        results: list[MemoryResult],
    ) -> str:
        if not results:
            return ""
        blocks: list[str] = []
        user_mems = [r for r in results if r.user_id and not r.session_id]
        session_mems = [r for r in results if r.session_id]
        if user_mems:
            blocks.append("## User Memory")
            for r in user_mems:
                blocks.append(f"- {r.text}")
        if session_mems:
            blocks.append("## Session Memory")
            for r in session_mems:
                blocks.append(f"- {r.text}")
        return "\n".join(blocks)

    @staticmethod
    def _extract_id(result: list[dict] | dict | str) -> str:
        if isinstance(result, str):
            return result
        if isinstance(result, list):
            if result and isinstance(result[0], dict):
                return result[0].get("id", str(result[0]))
            return str(result[0]) if result else ""
        if isinstance(result, dict):
            return result.get("id", str(result))
        return str(result)

    @staticmethod
    def _to_result(raw: dict) -> MemoryResult:
        text = raw.get("text") or raw.get("memory", "")
        score = raw.get("score") or raw.get("similarity") or 0.0
        if isinstance(score, (int, float)):
            score_f = float(score)
        else:
            score_f = 0.0
        meta = raw.get("metadata", {}) or {}
        return MemoryResult(
            memory_id=raw.get("id", ""),
            text=text,
            score=score_f,
            metadata=meta,
            user_id=meta.get("user_id"),
            session_id=meta.get("session_id"),
        )


def _estimate_tokens(text: str) -> int:
    return max(1, len(text) // 4)
