"""
Serveur MCP minimal — Python (FastAPI + SDK MCP).
Points d'entrée :
  - GET  /health    → Healthcheck K8s
  - POST /mcp       → Endpoint MCP (outils)
  - GET  /mcp/tools → Liste des outils disponibles
"""

import os
import json
import logging
from typing import Any

from fastapi import FastAPI, Request
from pydantic import BaseModel

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("mcp-server")

app = FastAPI(title="Pylos MCP Server (Python)", version="1.0.0")

SERVER_NAME = os.environ.get("MCP_SERVER_NAME", "python-mcp")
SERVER_TYPE = os.environ.get("MCP_SERVER_TYPE", "python")

# Outils MCP exposés par ce serveur
TOOLS = [
    {
        "name": "echo",
        "description": "Renvoie le message passé en paramètre",
        "input_schema": {
            "type": "object",
            "properties": {
                "message": {"type": "string", "description": "Message à renvoyer"}
            },
            "required": ["message"],
        },
    },
    {
        "name": "get_time",
        "description": "Renvoie l'heure actuelle",
        "input_schema": {
            "type": "object",
            "properties": {},
            "required": [],
        },
    },
    {
        "name": "list_files",
        "description": "Liste les fichiers dans un répertoire",
        "input_schema": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Chemin du répertoire"}
            },
            "required": ["path"],
        },
    },
]


class McpRequest(BaseModel):
    method: str
    params: dict[str, Any] | None = None
    id: str | int | None = None


class McpResponse(BaseModel):
    id: str | int | None = None
    result: dict[str, Any] | None = None
    error: dict[str, Any] | None = None


@app.get("/health")
async def health():
    return {"status": "ok", "server": SERVER_NAME, "type": SERVER_TYPE}


@app.get("/mcp/tools")
async def list_tools():
    return {"tools": TOOLS}


@app.post("/mcp")
async def handle_mcp(req: McpRequest):
    """
    Point d'entrée principal MCP.
    Reçoit une requête JSON-RPC et exécute l'outil demandé.
    """
    method = req.method
    params = req.params or {}

    result = await execute_tool(method, params)

    return McpResponse(id=req.id, result=result)


async def execute_tool(method: str, params: dict[str, Any]) -> dict[str, Any]:
    """
    Route la méthode MCP vers l'implémentation correspondante.
    """
    from datetime import datetime

    tool_map = {
        "echo": lambda p: {"message": p.get("message", "")},
        "get_time": lambda _: {
            "time": datetime.utcnow().isoformat(),
            "timezone": "UTC",
        },
        "list_files": lambda p: _list_files(p.get("path", ".")),
    }

    handler = tool_map.get(method)
    if not handler:
        raise ValueError(f"Unknown tool: {method}")

    return handler(params)


def _list_files(path: str) -> dict[str, Any]:
    import pathlib

    try:
        entries = list(pathlib.Path(path).iterdir())
        return {
            "files": [
                {"name": e.name, "is_dir": e.is_dir(), "size": e.stat().st_size}
                for e in entries
            ]
        }
    except Exception as e:
        return {"error": str(e)}
