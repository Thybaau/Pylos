from __future__ import annotations

from datetime import datetime
from pydantic import BaseModel, Field


class Mem0Entry(BaseModel):
    memory_id: str = Field(..., description="Unique memory identifier")
    text: str = Field(..., description="Memory text content")
    user_id: str | None = Field(None, description="Associated user ID")
    session_id: str | None = Field(None, description="Associated session ID")
    metadata: dict = Field(default_factory=dict, description="Extra metadata")
    score: float | None = Field(None, ge=0.0, le=1.0, description="Relevance score")
    created_at: datetime | None = Field(None, description="Creation timestamp")
    updated_at: datetime | None = Field(None, description="Last update timestamp")


class MemoryResult(BaseModel):
    memory_id: str
    text: str
    score: float
    metadata: dict = Field(default_factory=dict)
    user_id: str | None = None
    session_id: str | None = None


class UserProfile(BaseModel):
    user_id: str
    preferences: list[str] = Field(default_factory=list)
    habits: list[str] = Field(default_factory=list)
    facts: list[str] = Field(default_factory=list)
    memory_count: int = 0
    last_active: datetime | None = None


class AddMemoryRequest(BaseModel):
    user_id: str | None = None
    session_id: str | None = None
    data: str = Field(..., min_length=1, description="Memory content to store")
    metadata: dict = Field(default_factory=dict)
    role: str | None = Field(
        None, description="Context role: 'user', 'assistant', 'system'"
    )


class AddMemoryResponse(BaseModel):
    memory_id: str
    status: str = "stored"


class SearchRequest(BaseModel):
    query: str = Field(..., min_length=1)
    user_id: str | None = None
    session_id: str | None = None
    limit: int = Field(default=10, ge=1, le=100)


class ContextRequest(BaseModel):
    query: str = Field(..., min_length=1)
    user_id: str | None = None
    session_id: str | None = None
    max_tokens: int | None = None


class ContextResponse(BaseModel):
    context: str
    memories_used: int
    total_tokens: int


class UpdateMemoryRequest(BaseModel):
    data: str | None = None
    metadata: dict | None = None


class DeleteResponse(BaseModel):
    status: str = "deleted"
