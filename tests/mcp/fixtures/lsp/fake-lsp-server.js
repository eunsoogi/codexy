"use strict";

const fs = require("fs");

if (process.env.CODEXY_FAKE_LSP_EXIT_EARLY === "1") {
  process.exit(42);
}

let buffer = Buffer.alloc(0);
let nextDiagnostics = false;
let captureData = {};
let pendingSymbolRequest;
let pendingServerRequestId;
let openedUri = "";

function serverRequestIdFor(message) {
  if (process.env.CODEXY_FAKE_LSP_SERVER_REQUEST_ID === "match-client-request") {
    return message.id;
  }
  return 1000;
}

function encode(payload) {
  const body = Buffer.from(JSON.stringify(payload), "utf8");
  return Buffer.concat([Buffer.from(`Content-Length: ${body.length}\r\n\r\n`, "utf8"), body]);
}

function send(payload) {
  process.stdout.write(encode(payload));
}

function capture(message) {
  if (!process.env.CODEXY_FAKE_LSP_CAPTURE) return;
  captureData = {
    ...captureData,
    cwd: process.cwd(),
    rootUri: message.params.rootUri,
  };
  fs.writeFileSync(process.env.CODEXY_FAKE_LSP_CAPTURE, JSON.stringify(captureData, null, 2));
}

function captureUri(key, uri) {
  if (!process.env.CODEXY_FAKE_LSP_CAPTURE) return;
  captureData = { ...captureData, [key]: uri };
  fs.writeFileSync(process.env.CODEXY_FAKE_LSP_CAPTURE, JSON.stringify(captureData, null, 2));
}

function publishDiagnostics(uri = openedUri, message = "push-only diagnostic") {
  send({
    jsonrpc: "2.0",
    method: "textDocument/publishDiagnostics",
    params: {
      uri,
      diagnostics: [{ message, severity: 2, range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } } }],
    },
  });
}

function handle(message) {
  if (pendingSymbolRequest && message.id === pendingServerRequestId) {
    captureUri("serverRequestResponseId", message.id);
    captureUri("serverRequestResponseResult", message.result);
    send({ jsonrpc: "2.0", id: pendingSymbolRequest.id, result: [] });
    pendingSymbolRequest = undefined;
    pendingServerRequestId = undefined;
    return;
  }
  if (message.method === "initialize") {
    capture(message);
    send({
      jsonrpc: "2.0",
      id: message.id,
      result: { capabilities: process.env.CODEXY_FAKE_LSP_PULL_DIAGNOSTICS === "1" ? { diagnosticProvider: {} } : {} },
    });
    return;
  }
  if (message.method === "textDocument/didOpen") {
    openedUri = message.params.textDocument.uri;
    captureUri("openedUri", openedUri);
    nextDiagnostics = true;
    if (process.env.CODEXY_FAKE_LSP_UNRELATED_DIAGNOSTICS_FIRST === "1") {
      nextDiagnostics = false;
      publishDiagnostics("file:///workspace/unrelated.js", "unrelated diagnostic");
      setTimeout(() => publishDiagnostics(openedUri, "target diagnostic"), 50);
      return;
    }
    if (process.env.CODEXY_FAKE_LSP_PUSH_DIAGNOSTICS_ON_OPEN === "1") {
      nextDiagnostics = false;
      publishDiagnostics();
    }
    return;
  }
  if (message.method === "shutdown") {
    send({ jsonrpc: "2.0", id: message.id, result: null });
    return;
  }
  if (message.id !== undefined) {
    captureData = { ...captureData, requestMethods: [...(captureData.requestMethods || []), message.method] };
    if (process.env.CODEXY_FAKE_LSP_CAPTURE) {
      fs.writeFileSync(process.env.CODEXY_FAKE_LSP_CAPTURE, JSON.stringify(captureData, null, 2));
    }
    captureUri("requestUri", message.params?.textDocument?.uri);
    if (message.method === "textDocument/diagnostic" && process.env.CODEXY_FAKE_LSP_PULL_DIAGNOSTICS !== "1") {
      send({ jsonrpc: "2.0", id: message.id, error: { code: -32601, message: `Method not found: ${message.method}` } });
      return;
    }
    if (process.env.CODEXY_FAKE_LSP_REQUIRE_CLIENT_RESPONSE === "1") {
      const serverRequestId = serverRequestIdFor(message);
      pendingSymbolRequest = message;
      pendingServerRequestId = serverRequestId;
      send({
        jsonrpc: "2.0",
        id: serverRequestId,
        method: "workspace/configuration",
        params: { items: [{ section: "codexy.fake" }] },
      });
      return;
    }
    if (nextDiagnostics) {
      nextDiagnostics = false;
      publishDiagnostics();
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
