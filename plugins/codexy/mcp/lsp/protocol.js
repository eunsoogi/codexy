"use strict";

const fs = require("fs");
const path = require("path");
const { spawn } = require("child_process");
const {
  languageForPath,
  resolvePath,
  toFileUri,
} = require("./config");

const REQUEST_TIMEOUT_MS = 1500;
const STDERR_LIMIT = 4000;
const WORKSPACE_MARKERS = [
  ".git",
  "package.json",
  "pyproject.toml",
  "Cargo.toml",
  "go.mod",
  "deno.json",
  "tsconfig.json",
  "jsconfig.json",
];
const SUPPORTED_SERVER_REQUESTS = new Set([
  "client/registerCapability",
  "client/unregisterCapability",
  "window/workDoneProgress/create",
  "workspace/configuration",
]);

function encodeLsp(payload) {
  const body = Buffer.from(JSON.stringify(payload), "utf8");
  return Buffer.concat([Buffer.from(`Content-Length: ${body.length}\r\n\r\n`, "utf8"), body]);
}

function parseLspFrames(state, chunk, onMessage) {
  state.buffer = Buffer.concat([state.buffer, chunk]);
  while (true) {
    const headerEnd = state.buffer.indexOf("\r\n\r\n");
    if (headerEnd === -1) return;
    const header = state.buffer.subarray(0, headerEnd).toString("utf8");
    const match = /content-length:\s*(\d+)/i.exec(header);
    if (!match) throw new Error(`missing Content-Length header from LSP server: ${header}`);
    const length = Number(match[1]);
    const start = headerEnd + 4;
    const end = start + length;
    if (state.buffer.length < end) return;
    const body = state.buffer.subarray(start, end).toString("utf8");
    state.buffer = state.buffer.subarray(end);
    onMessage(JSON.parse(body));
  }
}

function workspaceRootForFile(filePath) {
  let directory;
  try {
    const stat = fs.statSync(filePath);
    directory = stat.isDirectory() ? filePath : path.dirname(filePath);
  } catch {
    directory = path.dirname(filePath);
  }
  while (true) {
    if (WORKSPACE_MARKERS.some((marker) => fs.existsSync(path.join(directory, marker)))) {
      return directory;
    }
    const parent = path.dirname(directory);
    if (parent === directory) return directory;
    directory = parent;
  }
}

async function runLspRequest({ server, filePath, method, params, timeoutMs = REQUEST_TIMEOUT_MS }) {
  const command = server.command;
  const absolutePath = resolvePath(filePath);
  const workspaceRoot = workspaceRootForFile(absolutePath);
  const child = spawn(command[0], command.slice(1), {
    cwd: workspaceRoot,
    env: process.env,
    stdio: ["pipe", "pipe", "pipe"],
  });
  let nextId = 1;
  let stderr = "";
  const pending = new Map();
  const notifications = [];
  const stdoutState = { buffer: Buffer.alloc(0) };

  function capStderr(chunk) {
    stderr = (stderr + chunk).slice(-STDERR_LIMIT);
  }

  function write(payload) {
    child.stdin.write(encodeLsp(payload));
  }

  function request(requestMethod, requestParams, requestTimeoutMs = timeoutMs) {
    const id = nextId++;
    const payload = { jsonrpc: "2.0", id, method: requestMethod, params: requestParams };
    const promise = new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        pending.delete(id);
        reject(new Error(`Timed out waiting for ${requestMethod}`));
      }, requestTimeoutMs);
      pending.set(id, { resolve, reject, timer });
    });
    write(payload);
    return promise;
  }

  function notification(notificationMethod, notificationParams) {
    write({ jsonrpc: "2.0", method: notificationMethod, params: notificationParams });
  }

  function respondToServerRequest(message) {
    if (SUPPORTED_SERVER_REQUESTS.has(message.method)) {
      write({ jsonrpc: "2.0", id: message.id, result: null });
      return;
    }
    write({
      jsonrpc: "2.0",
      id: message.id,
      error: { code: -32601, message: `Method not found: ${message.method}` },
    });
  }

  child.stdout.on("data", (chunk) => {
    try {
      parseLspFrames(stdoutState, chunk, (message) => {
        if (message.id !== undefined && pending.has(message.id)) {
          const waiter = pending.get(message.id);
          clearTimeout(waiter.timer);
          pending.delete(message.id);
          waiter.resolve(message);
          return;
        }
        if (message.id !== undefined && typeof message.method === "string") {
          respondToServerRequest(message);
          return;
        }
        notifications.push(message);
      });
    } catch (error) {
      capStderr(error instanceof Error ? error.message : String(error));
    }
  });
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", capStderr);
  child.on("exit", (code, signal) => {
    for (const [, waiter] of pending) {
      clearTimeout(waiter.timer);
      waiter.reject(new Error(`LSP server exited before response: code=${code} signal=${signal}`));
    }
    pending.clear();
  });

  try {
    const uri = toFileUri(absolutePath);
    const text = fs.readFileSync(absolutePath, "utf8");
    const initialize = await request("initialize", {
      processId: process.pid,
      rootUri: toFileUri(workspaceRoot),
      capabilities: {
        textDocument: {
          documentSymbol: { hierarchicalDocumentSymbolSupport: true },
          definition: { linkSupport: true },
          references: {},
          diagnostic: {},
          synchronization: { didSave: true },
        },
        workspace: {},
      },
      clientInfo: { name: "codexy-lsp-mcp", version: "0.1.0" },
    });
    if (initialize.error) {
      return { status: "error", server: { id: server.id }, error: initialize.error, stderr };
    }
    notification("initialized", {});
    notification("textDocument/didOpen", {
      textDocument: {
        uri,
        languageId: languageForPath(absolutePath, server),
        version: 1,
        text,
      },
    });
    const response = await request(method, params({ uri, absolutePath }), timeoutMs);
    if (response.error) {
      return { status: "error", server: { id: server.id }, error: response.error, stderr };
    }
    return {
      status: "ok",
      path: absolutePath,
      server: { id: server.id, executable: server.executable },
      result: response.result,
      diagnostics: notifications
        .filter((message) => message.method === "textDocument/publishDiagnostics")
        .map((message) => message.params),
      stderr,
    };
  } finally {
    for (const [, waiter] of pending) clearTimeout(waiter.timer);
    pending.clear();
    if (child.exitCode === null) {
      try {
        const shutdown = request("shutdown", null, 300);
        await shutdown.catch(() => {});
        notification("exit", {});
      } catch {
      }
    }
    if (child.exitCode === null) child.kill("SIGTERM");
    await new Promise((resolve) => {
      if (child.exitCode !== null) {
        resolve();
        return;
      }
      const timer = setTimeout(() => {
        if (child.exitCode === null) child.kill("SIGKILL");
        resolve();
      }, 300);
      child.once("exit", () => {
        clearTimeout(timer);
        resolve();
      });
    });
  }
}

module.exports = { runLspRequest };
