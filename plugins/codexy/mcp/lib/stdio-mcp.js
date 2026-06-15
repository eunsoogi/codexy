#!/usr/bin/env node
"use strict";

function createServer({ name, version = "0.1.0", tools, callTool }) {
  let buffer = Buffer.alloc(0);

  function send(payload) {
    const body = Buffer.from(JSON.stringify(payload), "utf8");
    process.stdout.write(`Content-Length: ${body.length}\r\n\r\n`);
    process.stdout.write(body);
  }

  function respond(id, result) {
    send({ jsonrpc: "2.0", id, result });
  }

  function fail(id, code, message) {
    send({ jsonrpc: "2.0", id, error: { code, message } });
  }

  async function handle(message) {
    const { id, method, params = {} } = message;
    try {
      if (method === "initialize") {
        respond(id, {
          protocolVersion: "2024-11-05",
          capabilities: { tools: {} },
          serverInfo: { name, version },
        });
      } else if (method === "notifications/initialized") {
        return;
      } else if (method === "tools/list") {
        respond(id, { tools });
      } else if (method === "tools/call") {
        respond(id, await callTool(params.name, params.arguments || {}));
      } else if (id !== undefined) {
        fail(id, -32601, `Unknown method: ${method}`);
      }
    } catch (error) {
      fail(id, -32000, error instanceof Error ? error.message : String(error));
    }
  }

  function parseOne() {
    const headerEnd = buffer.indexOf("\r\n\r\n");
    if (headerEnd === -1) return false;
    const header = buffer.subarray(0, headerEnd).toString("utf8");
    const match = /content-length:\s*(\d+)/i.exec(header);
    if (!match) throw new Error("Missing Content-Length header");
    const length = Number(match[1]);
    const start = headerEnd + 4;
    const end = start + length;
    if (buffer.length < end) return false;
    const raw = buffer.subarray(start, end).toString("utf8");
    buffer = buffer.subarray(end);
    void handle(JSON.parse(raw));
    return true;
  }

  process.stdin.on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    while (parseOne()) {
    }
  });
}

function textResult(text) {
  return { content: [{ type: "text", text }] };
}

module.exports = { createServer, textResult };
