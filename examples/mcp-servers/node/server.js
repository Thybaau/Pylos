/**
 * Serveur MCP minimal — Node.js (Express + SDK MCP).
 * Points d'entrée :
 *   - GET  /health    → Healthcheck K8s
 *   - POST /mcp       → Endpoint MCP (outils)
 *   - GET  /mcp/tools → Liste des outils disponibles
 */

import express from "express";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";

const PORT = parseInt(process.env.MCP_PORT || "8000", 10);
const SERVER_NAME = process.env.MCP_SERVER_NAME || "node-mcp";
const SERVER_TYPE = process.env.MCP_SERVER_TYPE || "node";

const app = express();
app.use(express.json());

// ---------------------------------------------------------------------------
// Outils MCP
// ---------------------------------------------------------------------------
const tools = [
  {
    name: "echo",
    description: "Renvoie le message passé en paramètre",
    inputSchema: {
      type: "object",
      properties: {
        message: { type: "string", description: "Message à renvoyer" },
      },
      required: ["message"],
    },
  },
  {
    name: "get_time",
    description: "Renvoie l'heure actuelle",
    inputSchema: {
      type: "object",
      properties: {},
      required: [],
    },
  },
  {
    name: "random_number",
    description: "Génère un nombre aléatoire",
    inputSchema: {
      type: "object",
      properties: {
        min: { type: "number", description: "Valeur minimale" },
        max: { type: "number", description: "Valeur maximale" },
      },
      required: [],
    },
  },
];

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

// Healthcheck K8s
app.get("/health", (_req, res) => {
  res.json({ status: "ok", server: SERVER_NAME, type: SERVER_TYPE });
});

// Liste des outils
app.get("/mcp/tools", (_req, res) => {
  res.json({ tools });
});

// Point d'entrée principal MCP
app.post("/mcp", async (req, res) => {
  try {
    const { method, params } = req.body;
    const result = await executeTool(method, params || {});
    res.json({ result });
  } catch (err) {
    res.status(400).json({
      error: {
        message: err instanceof Error ? err.message : "Unknown error",
      },
    });
  }
});

// ---------------------------------------------------------------------------
// Routeur d'outils
// ---------------------------------------------------------------------------
async function executeTool(method, params) {
  switch (method) {
    case "echo":
      return { message: params.message || "" };

    case "get_time":
      return {
        time: new Date().toISOString(),
        timezone: "UTC",
      };

    case "random_number": {
      const min = params.min ?? 0;
      const max = params.max ?? 100;
      return {
        value: Math.floor(Math.random() * (max - min + 1)) + min,
      };
    }

    default:
      throw new Error(`Unknown tool: ${method}`);
  }
}

// ---------------------------------------------------------------------------
// Démarrage
// ---------------------------------------------------------------------------
app.listen(PORT, () => {
  console.log(
    JSON.stringify({
      event: "mcp_server_started",
      server: SERVER_NAME,
      type: SERVER_TYPE,
      port: PORT,
    })
  );
});
