"use strict";

const { spawn } = require("child_process");
const assert = require("node:assert/strict");

function encodeMessage(payload) {
  const body = Buffer.from(JSON.stringify(payload), "utf8");
  return Buffer.concat([
    Buffer.from(`Content-Length: ${body.length}\r\n\r\n`, "utf8"),
    body,
  ]);
}

function parseFrames(state, chunk, onMessage) {
  state.buffer = Buffer.concat([state.buffer, chunk]);
  while (true) {
    const headerEnd = state.buffer.indexOf("\r\n\r\n");
    if (headerEnd === -1) return;

    const header = state.buffer.subarray(0, headerEnd).toString("utf8");
    const match = /content-length:\s*(\d+)/i.exec(header);
    assert.ok(match, `Missing Content-Length header in ${JSON.stringify(header)}`);

    const length = Number(match[1]);
    const start = headerEnd + 4;
    const end = start + length;
    if (state.buffer.length < end) return;

    const body = state.buffer.subarray(start, end).toString("utf8");
    state.buffer = state.buffer.subarray(end);
    onMessage(JSON.parse(body));
  }
}

function createStdioClient(command, args = [], options = {}) {
  const child = spawn(command, args, {
    cwd: options.cwd,
    env: { ...process.env, ...(options.env || {}) },
    stdio: ["pipe", "pipe", "pipe"],
  });

  let nextId = 1;
  const pending = new Map();
  const messages = [];
  const stderr = [];
  const stdoutState = { buffer: Buffer.alloc(0) };

  child.stdout.on("data", (chunk) => {
    parseFrames(stdoutState, chunk, (message) => {
      messages.push(message);
      const waiter = pending.get(message.id);
      if (!waiter) return;
      clearTimeout(waiter.timer);
      pending.delete(message.id);
      waiter.resolve(message);
    });
  });

  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => stderr.push(chunk));

  child.on("exit", (code, signal) => {
    for (const [id, waiter] of pending) {
      clearTimeout(waiter.timer);
      waiter.reject(
        new Error(`MCP child exited before response ${id}: code=${code} signal=${signal} stderr=${stderr.join("")}`)
      );
    }
    pending.clear();
  });

  function request(method, params, timeoutMs = 1500) {
    const id = nextId++;
    const payload = { jsonrpc: "2.0", id, method, params };
    const response = new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        pending.delete(id);
        reject(new Error(`Timed out waiting for ${method} response ${id}; stderr=${stderr.join("")}`));
      }, timeoutMs);
      pending.set(id, { resolve, reject, timer });
    });
    child.stdin.write(encodeMessage(payload));
    return response;
  }

  async function initialize() {
    return request("initialize", {
      protocolVersion: "2024-11-05",
      capabilities: {},
      clientInfo: { name: "codexy-test-client", version: "0.0.0" },
    });
  }

  async function listTools() {
    const response = await request("tools/list", {});
    if (response.error) return response;
    return response.result.tools;
  }

  async function callTool(name, args = {}, timeoutMs) {
    return request("tools/call", { name, arguments: args }, timeoutMs);
  }

  async function close() {
    for (const [, waiter] of pending) clearTimeout(waiter.timer);
    pending.clear();
    if (child.exitCode !== null) return;
    child.stdin.end();
    child.kill("SIGTERM");
    await new Promise((resolve) => {
      const timer = setTimeout(() => {
        if (child.exitCode === null) child.kill("SIGKILL");
        resolve();
      }, 500);
      child.once("exit", () => {
        clearTimeout(timer);
        resolve();
      });
    });
  }

  return {
    child,
    stderr,
    messages,
    request,
    initialize,
    listTools,
    callTool,
    close,
  };
}

function textContent(response) {
  assert.ifError(response.error);
  assert.ok(response.result, "expected a JSON-RPC result");
  assert.ok(Array.isArray(response.result.content), "expected MCP content array");
  const text = response.result.content
    .filter((entry) => entry.type === "text")
    .map((entry) => entry.text)
    .join("\n");
  assert.notEqual(text, "", "expected non-empty text content");
  return text;
}

function jsonTextContent(response) {
  return JSON.parse(textContent(response));
}

module.exports = {
  createStdioClient,
  jsonTextContent,
  textContent,
};
