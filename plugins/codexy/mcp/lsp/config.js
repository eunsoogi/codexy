"use strict";

const fs = require("fs");
const path = require("path");
const { pathToFileURL } = require("url");

const pluginRoot = path.resolve(__dirname, "..", "..");
const repoRoot = path.resolve(pluginRoot, "..", "..");
const lspConfigPath = path.join(pluginRoot, ".codex", "lsp-client.json");
const lspCatalogPath = path.join(pluginRoot, "lsp", "server-catalog.toml");

const LANGUAGE_BY_EXTENSION = {
  ".js": "javascript",
  ".jsx": "javascriptreact",
  ".mjs": "javascript",
  ".cjs": "javascript",
  ".ts": "typescript",
  ".tsx": "typescriptreact",
  ".mts": "typescript",
  ".cts": "typescript",
  ".json": "json",
  ".jsonc": "jsonc",
  ".py": "python",
  ".pyi": "python",
  ".rs": "rust",
  ".go": "go",
  ".md": "markdown",
  ".markdown": "markdown",
  ".yaml": "yaml",
  ".yml": "yaml",
  ".toml": "toml",
  ".sh": "shellscript",
  ".bash": "shellscript",
  ".zsh": "shellscript",
  ".ksh": "shellscript",
};

function readConfig() {
  return JSON.parse(fs.readFileSync(lspConfigPath, "utf8")).lsp || {};
}

function readCatalog() {
  const catalog = new Map();
  let current = null;
  for (const rawLine of fs.readFileSync(lspCatalogPath, "utf8").split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) continue;
    if (line === "[[servers]]") {
      current = {};
      continue;
    }
    if (!current) continue;
    const match = /^([A-Za-z0-9_-]+)\s*=\s*(.+)$/.exec(line);
    if (!match) continue;
    const [, key, value] = match;
    if (value.startsWith('"')) {
      current[key] = JSON.parse(value);
    } else if (value.startsWith("[")) {
      current[key] = JSON.parse(value);
    }
    if (current.id) catalog.set(current.id, current);
  }
  return catalog;
}

function normalizeExt(filePath) {
  const base = path.basename(filePath);
  if (base === "Dockerfile") return "Dockerfile";
  return path.extname(base);
}

function languageForPath(filePath, server) {
  const ext = normalizeExt(filePath);
  if (LANGUAGE_BY_EXTENSION[ext]) return LANGUAGE_BY_EXTENSION[ext];
  if (ext === "Dockerfile") return "dockerfile";
  return String(server.language || server.id || ext.replace(/^\./, "") || "plaintext").toLowerCase();
}

function resolvePath(filePath, root) {
  if (path.isAbsolute(filePath)) return filePath;
  const base = root ? path.resolve(root) : process.cwd();
  return path.resolve(base, filePath);
}

function toFileUri(filePath, root) {
  return pathToFileURL(resolvePath(filePath, root)).href;
}

function resolveExecutable(command) {
  if (!Array.isArray(command) || command.length === 0 || typeof command[0] !== "string") {
    return { available: false, reason: "server command is missing" };
  }
  const executable = command[0];
  if (executable.includes(path.sep)) {
    return fs.existsSync(executable)
      ? { available: true, executable }
      : { available: false, reason: `executable not found: ${executable}` };
  }
  for (const entry of (process.env.PATH || "").split(path.delimiter)) {
    if (!entry) continue;
    const candidate = path.join(entry, executable);
    try {
      fs.accessSync(candidate, fs.constants.X_OK);
      return { available: true, executable: candidate };
    } catch {
    }
  }
  return { available: false, reason: `executable not found on PATH: ${executable}` };
}

function enrichServer(id, config, catalog = readCatalog()) {
  const catalogEntry = catalog.get(id) || {};
  const command = Array.isArray(config.command) ? config.command : catalogEntry.command;
  const server = { id, ...catalogEntry, ...config, command };
  const availability = resolveExecutable(command);
  const installHints = [catalogEntry.install, config.install].filter(Boolean);
  return {
    ...server,
    executable: Array.isArray(command) ? command[0] : undefined,
    resolvedExecutable: availability.executable,
    available: availability.available,
    unavailableReason: availability.reason,
    installHints,
  };
}

function matchingServers(filePath) {
  const ext = normalizeExt(filePath);
  const catalog = readCatalog();
  return Object.entries(readConfig())
    .filter(([, config]) => Array.isArray(config.extensions) && config.extensions.includes(ext))
    .sort((a, b) => (b[1].priority || 0) - (a[1].priority || 0))
    .map(([id, config]) => enrichServer(id, config, catalog));
}

function serverFromOverride(override) {
  if (!override || typeof override !== "object") return null;
  if (!override.id) throw new Error("server.id is required when server override is provided");
  return enrichServer(override.id, override, readCatalog());
}

function selectServer(args) {
  const override = serverFromOverride(args.server);
  if (override) return override;
  const matches = matchingServers(args.path);
  if (matches.length === 0) {
    return {
      id: "unmatched",
      available: false,
      installHints: [],
      unavailableReason: `no LSP server matches ${normalizeExt(args.path) || path.basename(args.path)}`,
    };
  }
  return matches[0];
}

function unavailablePayload(filePath, server, root) {
  return {
    status: "unavailable",
    path: filePath ? resolvePath(filePath, root) : undefined,
    server: { id: server.id, executable: server.executable, command: server.command },
    reason: server.unavailableReason || "server executable unavailable",
    installHints: server.installHints || [],
  };
}

module.exports = {
  languageForPath,
  matchingServers,
  normalizeExt,
  readConfig,
  repoRoot,
  resolvePath,
  selectServer,
  toFileUri,
  unavailablePayload,
};
