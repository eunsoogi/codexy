#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");
const { createServer, textResult } = require("../lib/stdio-mcp");

const pluginRoot = path.resolve(__dirname, "..", "..");
const lspConfigPath = path.join(pluginRoot, ".codex", "lsp-client.json");

function readConfig() {
  return JSON.parse(fs.readFileSync(lspConfigPath, "utf8")).lsp || {};
}

function normalizeExt(filePath) {
  const base = path.basename(filePath);
  if (base === "Dockerfile") return "Dockerfile";
  return path.extname(base);
}

function matchingServers(filePath) {
  const ext = normalizeExt(filePath);
  return Object.entries(readConfig())
    .filter(([, config]) => Array.isArray(config.extensions) && config.extensions.includes(ext))
    .sort((a, b) => (b[1].priority || 0) - (a[1].priority || 0))
    .map(([id, config]) => ({ id, ...config }));
}

const tools = [
  {
    name: "lsp_list_servers",
    description: "List Codexy LSP client server registrations and covered file extensions.",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "lsp_for_path",
    description: "Return the Codexy LSP server registrations that match a file path.",
    inputSchema: {
      type: "object",
      properties: { path: { type: "string", description: "Repository-relative or absolute file path." } },
      required: ["path"],
    },
  },
];

async function callTool(name, args) {
  if (name === "lsp_list_servers") {
    return textResult(JSON.stringify(readConfig(), null, 2));
  }
  if (name === "lsp_for_path") {
    if (!args.path) throw new Error("path is required");
    return textResult(JSON.stringify(matchingServers(args.path), null, 2));
  }
  throw new Error(`Unknown tool: ${name}`);
}

createServer({ name: "codexy-lsp", tools, callTool });
