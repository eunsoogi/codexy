"use strict";

const fs = require("fs");

let buffer = Buffer.alloc(0);
let nextDiagnostics = false;

function encode(payload) {
  const body = Buffer.from(JSON.stringify(payload), "utf8");
  return Buffer.concat([Buffer.from(`Content-Length: ${body.length}\r\n\r\n`, "utf8"), body]);
}

function send(payload) {
  process.stdout.write(encode(payload));
}

function capture(message) {
  if (!process.env.CODEXY_FAKE_LSP_CAPTURE) return;
  fs.writeFileSync(process.env.CODEXY_FAKE_LSP_CAPTURE, JSON.stringify({
    cwd: process.cwd(),
    rootUri: message.params.rootUri,
  }, null, 2));
}

function handle(message) {
  if (message.method === "initialize") {
    capture(message);
    send({ jsonrpc: "2.0", id: message.id, result: { capabilities: {} } });
    return;
  }
  if (message.method === "textDocument/didOpen") {
    nextDiagnostics = true;
    return;
  }
  if (message.method === "shutdown") {
    send({ jsonrpc: "2.0", id: message.id, result: null });
    return;
  }
  if (message.id !== undefined) {
    if (nextDiagnostics) {
      nextDiagnostics = false;
      send({ jsonrpc: "2.0", method: "textDocument/publishDiagnostics", params: { uri: "", diagnostics: [] } });
    }
    send({ jsonrpc: "2.0", id: message.id, result: [] });
  }
}

function parse(chunk) {
  buffer = Buffer.concat([buffer, chunk]);
  while (true) {
    const headerEnd = buffer.indexOf("\r\n\r\n");
    if (headerEnd === -1) return;
    const header = buffer.subarray(0, headerEnd).toString("utf8");
    const match = /content-length:\s*(\d+)/i.exec(header);
    if (!match) throw new Error(`Missing Content-Length header: ${header}`);
    const start = headerEnd + 4;
    const end = start + Number(match[1]);
    if (buffer.length < end) return;
    const body = buffer.subarray(start, end).toString("utf8");
    buffer = buffer.subarray(end);
    handle(JSON.parse(body));
  }
}

process.stdin.on("data", parse);
