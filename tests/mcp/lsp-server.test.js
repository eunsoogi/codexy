"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const path = require("path");
const { createStdioClient, jsonTextContent } = require("./stdio-client");

const repoRoot = path.resolve(__dirname, "..", "..");
const lspServer = path.join(repoRoot, "plugins/codexy/mcp/lsp/server.js");
const jsFixture = path.join(repoRoot, "tests/mcp/fixtures/lsp/sample.js");

async function withLspClient(fn) {
  const client = createStdioClient(process.execPath, [lspServer], { cwd: repoRoot });
  try {
    await client.initialize();
    return await fn(client);
  } finally {
    await client.close();
  }
}

function assertStructuredToolResult(response) {
  assert.ifError(response.error);
  return jsonTextContent(response);
}

test("lsp_status reports availability and install hints for a JavaScript path", async () => {
  await withLspClient(async (client) => {
    const response = await client.callTool("lsp_status", { path: jsFixture });
    const status = assertStructuredToolResult(response);

    assert.equal(status.path, jsFixture);
    assert.equal(status.language, "javascript");
    assert.equal(status.server.id, "typescript");
    assert.match(status.server.executable, /typescript-language-server/);
    assert.equal(typeof status.available, "boolean");
    assert.ok(Array.isArray(status.installHints));
    assert.ok(status.installHints.length > 0);
  });
});

test("full LSP operation tools are registered", async () => {
  await withLspClient(async (client) => {
    const tools = await client.listTools();
    const names = tools.map((tool) => tool.name);

    assert.ok(names.includes("lsp_document_symbols"));
    assert.ok(names.includes("lsp_definition"));
    assert.ok(names.includes("lsp_references"));
    assert.ok(names.includes("lsp_diagnostics"));
  });
});

test("LSP operations return structured unavailable when the server executable is missing", async () => {
  await withLspClient(async (client) => {
    const response = await client.callTool("lsp_document_symbols", {
      path: jsFixture,
      server: {
        id: "missing-test-server",
        command: ["definitely-not-a-real-language-server", "--stdio"],
      },
    });
    const payload = assertStructuredToolResult(response);

    assert.equal(payload.status, "unavailable");
    assert.equal(payload.server.id, "missing-test-server");
    assert.match(payload.reason, /not found|missing|unavailable/i);
    assert.ok(Array.isArray(payload.installHints));
  });
});
