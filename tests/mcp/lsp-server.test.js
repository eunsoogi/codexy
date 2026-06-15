"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { pathToFileURL } = require("url");
const { createStdioClient, jsonTextContent } = require("./stdio-client");

const repoRoot = path.resolve(__dirname, "..", "..");
const pluginRoot = path.join(repoRoot, "plugins/codexy");
const lspServer = path.join(repoRoot, "plugins/codexy/mcp/lsp/server.js");
const jsFixture = path.join(repoRoot, "tests/mcp/fixtures/lsp/sample.js");
const externalWorkspace = path.join(repoRoot, "tests/mcp/fixtures/lsp/external-workspace");
const externalFixture = path.join(externalWorkspace, "src/sample.js");
const fakeLspServer = path.join(repoRoot, "tests/mcp/fixtures/lsp/fake-lsp-server.js");

async function withLspClient(fn, options = {}) {
  const client = createStdioClient(process.execPath, [lspServer], { cwd: options.cwd || repoRoot, env: options.env });
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
    const statusTool = tools.find((tool) => tool.name === "lsp_status");
    const symbolTool = tools.find((tool) => tool.name === "lsp_document_symbols");

    assert.ok(names.includes("lsp_document_symbols"));
    assert.ok(names.includes("lsp_definition"));
    assert.ok(names.includes("lsp_references"));
    assert.ok(names.includes("lsp_diagnostics"));
    assert.ok(statusTool.inputSchema.properties.root);
    assert.ok(symbolTool.inputSchema.properties.root);
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

test("LSP operations return structured unavailable when a path command is not executable", async () => {
  const commandPath = path.join(os.tmpdir(), `codexy-non-executable-lsp-${process.pid}-${Date.now()}`);
  try {
    fs.writeFileSync(commandPath, "#!/bin/sh\nexit 0\n", { mode: 0o600 });

    await withLspClient(async (client) => {
      const response = await client.callTool("lsp_document_symbols", {
        path: jsFixture,
        server: {
          id: "non-executable-test-server",
          command: [commandPath, "--stdio"],
        },
      });
      const payload = assertStructuredToolResult(response);

      assert.equal(payload.status, "unavailable");
      assert.equal(payload.server.id, "non-executable-test-server");
      assert.match(payload.reason, /not executable|permission|unavailable/i);
      assert.ok(Array.isArray(payload.installHints));
    });
  } finally {
    fs.rmSync(commandPath, { force: true });
  }
});

test("LSP operations initialize servers from the target file workspace root", async () => {
  const capturePath = path.join(os.tmpdir(), `codexy-fake-lsp-${process.pid}-${Date.now()}.json`);
  try {
    await withLspClient(async (client) => {
      const response = await client.callTool("lsp_document_symbols", {
        path: externalFixture,
        server: {
          id: "fake-lsp",
          command: [process.execPath, fakeLspServer],
        },
      });
      const payload = assertStructuredToolResult(response);

      assert.equal(payload.status, "ok");
      assert.equal(payload.path, externalFixture);
    }, { env: { CODEXY_FAKE_LSP_CAPTURE: capturePath } });

    const capture = JSON.parse(fs.readFileSync(capturePath, "utf8"));
    assert.equal(capture.cwd, externalWorkspace);
    assert.equal(capture.rootUri, pathToFileURL(externalWorkspace).href);
  } finally {
    fs.rmSync(capturePath, { force: true });
  }
});

test("LSP operations resolve relative paths against caller root", async () => {
  const capturePath = path.join(os.tmpdir(), `codexy-fake-lsp-root-${process.pid}-${Date.now()}.json`);
  try {
    await withLspClient(async (client) => {
      const statusResponse = await client.callTool("lsp_status", {
        path: "src/sample.js",
        root: externalWorkspace,
      });
      const status = assertStructuredToolResult(statusResponse);
      assert.equal(status.path, externalFixture);

      const response = await client.callTool("lsp_document_symbols", {
        path: "src/sample.js",
        root: externalWorkspace,
        server: {
          id: "fake-lsp",
          command: [process.execPath, fakeLspServer],
        },
      });
      const payload = assertStructuredToolResult(response);
      const externalUri = pathToFileURL(externalFixture).href;

      assert.equal(payload.status, "ok");
      assert.equal(payload.path, externalFixture);

      const capture = JSON.parse(fs.readFileSync(capturePath, "utf8"));
      assert.equal(capture.cwd, externalWorkspace);
      assert.equal(capture.rootUri, pathToFileURL(externalWorkspace).href);
      assert.equal(capture.openedUri, externalUri);
      assert.equal(capture.requestUri, externalUri);
    }, { cwd: pluginRoot, env: { CODEXY_FAKE_LSP_CAPTURE: capturePath } });
  } finally {
    fs.rmSync(capturePath, { force: true });
  }
});

test("LSP operations answer server-to-client requests before document symbols", async () => {
  const capturePath = path.join(os.tmpdir(), `codexy-fake-lsp-server-request-${process.pid}-${Date.now()}.json`);
  try {
    await withLspClient(async (client) => {
      const response = await client.callTool("lsp_document_symbols", {
        path: externalFixture,
        server: {
          id: "fake-lsp",
          command: [process.execPath, fakeLspServer],
        },
      });
      const payload = assertStructuredToolResult(response);

      assert.equal(payload.status, "ok");
      assert.equal(payload.path, externalFixture);

      const capture = JSON.parse(fs.readFileSync(capturePath, "utf8"));
      assert.equal(capture.serverRequestResponseId, 1000);
    }, {
      env: {
        CODEXY_FAKE_LSP_CAPTURE: capturePath,
        CODEXY_FAKE_LSP_REQUIRE_CLIENT_RESPONSE: "1",
      },
    });
  } finally {
    fs.rmSync(capturePath, { force: true });
  }
});
