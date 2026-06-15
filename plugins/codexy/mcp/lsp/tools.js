"use strict";

const { textResult } = require("../lib/stdio-mcp");
const {
  languageForPath,
  matchingServers,
  normalizeExt,
  readConfig,
  resolvePath,
  selectServer,
  unavailablePayload,
} = require("./config");
const { runLspRequest } = require("./protocol");

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
      properties: {
        path: { type: "string", description: "Absolute or relative file path used for language/server matching." },
        root: { type: "string", description: "Optional workspace root when you want relative paths normalized before matching." },
        workspaceRoot: { type: "string", description: "Alias for root." },
      },
      required: ["path"],
    },
  },
  {
    name: "lsp_status",
    description: "Report the configured Codexy LSP server, PATH availability, and install hints for a file path.",
    inputSchema: {
      type: "object",
      properties: {
        path: { type: "string", description: "Absolute file path, or workspace-relative path when root is provided." },
        root: { type: "string", description: "Workspace root used to resolve a relative path." },
        workspaceRoot: { type: "string", description: "Alias for root." },
        server: { type: "object", description: "Optional server override with id and command array." },
      },
      required: ["path"],
    },
  },
  {
    name: "lsp_document_symbols",
    description: "Open a file through the matching LSP server and request document symbols.",
    inputSchema: {
      type: "object",
      properties: {
        path: { type: "string", description: "Absolute file path, or workspace-relative path when root is provided." },
        root: { type: "string", description: "Workspace root used to resolve a relative path." },
        workspaceRoot: { type: "string", description: "Alias for root." },
        server: { type: "object" },
      },
      required: ["path"],
    },
  },
  {
    name: "lsp_definition",
    description: "Open a file through the matching LSP server and request a definition at a position.",
    inputSchema: {
      type: "object",
      properties: {
        path: { type: "string", description: "Absolute file path, or workspace-relative path when root is provided." },
        root: { type: "string", description: "Workspace root used to resolve a relative path." },
        workspaceRoot: { type: "string", description: "Alias for root." },
        line: { type: "number" },
        character: { type: "number" },
        server: { type: "object" },
      },
      required: ["path"],
    },
  },
  {
    name: "lsp_references",
    description: "Open a file through the matching LSP server and request references at a position.",
    inputSchema: {
      type: "object",
      properties: {
        path: { type: "string", description: "Absolute file path, or workspace-relative path when root is provided." },
        root: { type: "string", description: "Workspace root used to resolve a relative path." },
        workspaceRoot: { type: "string", description: "Alias for root." },
        line: { type: "number" },
        character: { type: "number" },
        includeDeclaration: { type: "boolean" },
        server: { type: "object" },
      },
      required: ["path"],
    },
  },
  {
    name: "lsp_diagnostics",
    description: "Open a file through the matching LSP server and request diagnostics.",
    inputSchema: {
      type: "object",
      properties: {
        path: { type: "string", description: "Absolute file path, or workspace-relative path when root is provided." },
        root: { type: "string", description: "Workspace root used to resolve a relative path." },
        workspaceRoot: { type: "string", description: "Alias for root." },
        server: { type: "object" },
      },
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
    return textResult(JSON.stringify(matchingServers(matchPathFromArgs(args)), null, 2));
  }
  if (name === "lsp_status") {
    if (!args.path) throw new Error("path is required");
    const filePath = resolvePath(args.path, rootFromArgs(args));
    const server = selectServer({ ...args, path: filePath });
    return textResult(JSON.stringify({
      path: filePath,
      language: languageForPath(filePath, server),
      extension: normalizeExt(filePath),
      server: {
        id: server.id,
        language: server.language,
        command: server.command,
        executable: server.executable,
        resolvedExecutable: server.resolvedExecutable,
      },
      available: Boolean(server.available),
      installHints: server.installHints || [],
      reason: server.available ? undefined : server.unavailableReason,
    }, null, 2));
  }
  if (name === "lsp_document_symbols") {
    return operationResult(args, "textDocument/documentSymbol", ({ uri }) => ({ textDocument: { uri } }));
  }
  if (name === "lsp_definition") {
    return operationResult(args, "textDocument/definition", ({ uri }) => ({
      textDocument: { uri },
      position: { line: args.line || 0, character: args.character || 0 },
    }));
  }
  if (name === "lsp_references") {
    return operationResult(args, "textDocument/references", ({ uri }) => ({
      textDocument: { uri },
      position: { line: args.line || 0, character: args.character || 0 },
      context: { includeDeclaration: args.includeDeclaration !== false },
    }));
  }
  if (name === "lsp_diagnostics") {
    return operationResult(args, "textDocument/diagnostic", ({ uri }) => ({ textDocument: { uri } }));
  }
  throw new Error(`Unknown tool: ${name}`);
}

async function operationResult(args, method, params) {
  if (!args.path) throw new Error("path is required");
  const filePath = resolvePath(args.path, rootFromArgs(args));
  const server = selectServer({ ...args, path: filePath });
  if (!server.available) {
    return textResult(JSON.stringify(unavailablePayload(filePath, server, rootFromArgs(args)), null, 2));
  }
  try {
    const result = await runLspRequest({ server, filePath, method, params });
    return textResult(JSON.stringify(result, null, 2));
  } catch (error) {
    return textResult(JSON.stringify({
      status: "error",
      path: filePath,
      server: { id: server.id, executable: server.executable },
      reason: error instanceof Error ? error.message : String(error),
      installHints: server.installHints || [],
    }, null, 2));
  }
}

function rootFromArgs(args) {
  return args.root || args.workspaceRoot;
}

function matchPathFromArgs(args) {
  const root = rootFromArgs(args);
  return root ? resolvePath(args.path, root) : args.path;
}

module.exports = { callTool, tools };
