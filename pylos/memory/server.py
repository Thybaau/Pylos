from __future__ import annotations

import logging
from contextlib import asynccontextmanager

import uvicorn
from fastapi import FastAPI, HTTPException

from pylos.memory.config import Mem0Settings
from pylos.memory.manager import PylosMemoryManager
from pylos.memory.models import (
    AddMemoryRequest,
    AddMemoryResponse,
    ContextRequest,
    ContextResponse,
    DeleteResponse,
    SearchRequest,
    UpdateMemoryRequest,
)

log = logging.getLogger(__name__)

memory_manager: PylosMemoryManager | None = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    global memory_manager
    settings = Mem0Settings.from_env()
    _configure_logging(settings.log_level)
    memory_manager = PylosMemoryManager(config=settings)
    log.info(
        "Mem0 sidecar started on %s:%d",
        settings.sidecar_host,
        settings.sidecar_port,
    )
    yield
    log.info("Mem0 sidecar shutting down")


app = FastAPI(
    title="Pylos Mem0 Sidecar",
    version="1.0.0",
    description="Memory management sidecar for Pylos using Mem0",
    lifespan=lifespan,
)


def _get_manager() -> PylosMemoryManager:
    if memory_manager is None:
        raise HTTPException(status_code=503, detail="Memory manager not initialized")
    return memory_manager


@app.get("/health")
async def health():
    return {"status": "ok"}


@app.post("/api/memory/user", response_model=AddMemoryResponse)
async def add_user_memory(req: AddMemoryRequest):
    if not req.user_id:
        raise HTTPException(status_code=422, detail="user_id is required for user memory")
    mgr = _get_manager()
    memory_id = mgr.add_user_memory(
        user_id=req.user_id,
        data=req.data,
        metadata=req.metadata,
    )
    return AddMemoryResponse(memory_id=memory_id)


@app.post("/api/memory/session", response_model=AddMemoryResponse)
async def add_session_memory(req: AddMemoryRequest):
    if not req.user_id:
        raise HTTPException(status_code=422, detail="user_id is required")
    if not req.session_id:
        raise HTTPException(status_code=422, detail="session_id is required for session memory")
    mgr = _get_manager()
    memory_id = mgr.add_session_memory(
        session_id=req.session_id,
        user_id=req.user_id,
        data=req.data,
        metadata=req.metadata,
    )
    return AddMemoryResponse(memory_id=memory_id)


@app.get("/api/memory/search")
async def search_memory(
    query: str,
    user_id: str | None = None,
    session_id: str | None = None,
    limit: int = 10,
):
    mgr = _get_manager()
    results = mgr.search(query=query, user_id=user_id, session_id=session_id, limit=limit)
    return {"results": [r.model_dump() for r in results]}


@app.get("/api/memory/context", response_model=ContextResponse)
async def get_context(
    query: str,
    user_id: str | None = None,
    session_id: str | None = None,
    max_tokens: int | None = None,
):
    mgr = _get_manager()
    context, count, tokens = mgr.get_relevant_context(
        query=query,
        user_id=user_id,
        session_id=session_id,
        max_tokens=max_tokens,
    )
    return ContextResponse(context=context, memories_used=count, total_tokens=tokens)


@app.put("/api/memory/{memory_id}")
async def update_memory(memory_id: str, req: UpdateMemoryRequest):
    mgr = _get_manager()
    ok = mgr.update_memory(
        memory_id=memory_id,
        data=req.data,
        metadata=req.metadata,
    )
    if not ok:
        raise HTTPException(status_code=404, detail="Memory not found or update failed")
    return {"status": "updated"}


@app.delete("/api/memory/{memory_id}", response_model=DeleteResponse)
async def delete_memory(memory_id: str):
    mgr = _get_manager()
    ok = mgr.delete_memory(memory_id=memory_id)
    if not ok:
        raise HTTPException(status_code=404, detail="Memory not found")
    return DeleteResponse()


@app.get("/api/memory/user/{user_id}/profile")
async def get_user_profile(user_id: str):
    mgr = _get_manager()
    profile = mgr.get_user_profile(user_id=user_id)
    return profile.model_dump()


def _configure_logging(level: str):
    logging.basicConfig(
        level=getattr(logging, level.upper(), logging.INFO),
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    )


def main():
    settings = Mem0Settings.from_env()
    _configure_logging(settings.log_level)
    uvicorn.run(
        "pylos.memory.server:app",
        host=settings.sidecar_host,
        port=settings.sidecar_port,
        log_level=settings.log_level.lower(),
    )


if __name__ == "__main__":
    main()
