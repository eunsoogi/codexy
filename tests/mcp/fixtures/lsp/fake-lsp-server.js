"use strict";

const fs = require("fs");

let buffer = Buffer.alloc(0);
let nextDiagnostics = false;
let captureData = {};
let pendingSymbolRequest;
let serverRequestId = 1000;

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

function handle(message) {
  if (pendingSymbolRequest && message.id === serverRequestId) {
    captureUri("serverRequestResponseId", message.id);
    captureUri("serverRequestResponseResult", message.result);
    send({ jsonrpc: "2.0", id: pendingSymbolRequest.id, result: [] });
    pendingSymbolRequest = undefined;
    return;
  }
  if (message.method === "initialize") {
    capture(message);
    send({ jsonrpc: "2.0", id: message.id, result: { capabilities: {} } });
    return;
  }
  if (message.method === "textDocument/didOpen") {
    captureUri("openedUri", message.params.textDocument.uri);
    nextDiagnostics = true;
    return;
  }
  if (message.method === "shutdown") {
    send({ jsonrpc: "2.0", id: message.id, result: null });
    return;
  }
  if (message.id !== undefined) {
    captureUri("requestUri", message.params?.textDocument?.uri);
    if (process.env.CODEXY_FAKE_LSP_REQUIRE_CLIENT_RESPONSE === "1") {
      pendingSymbolRequest = message;
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
