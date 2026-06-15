"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { fileURLToPath, pathToFileURL } = require("url");
const { fakeLspCommand, readCapture, withFakeLspCapture, withMarkerlessWorkspace, withWorkspaceRelativeFakeLsp } = require("./fixtures/lsp/fake-lsp-fixtures");
const { assertStructuredToolResult, withLspClient } = require("./fixtures/lsp/test-harness");
const repoRoot = path.resolve(__dirname, "..", "..");
const pluginRoot = path.join(repoRoot, "plugins/codexy");
const lspServer = path.join(repoRoot, "plugins/codexy/mcp/lsp/server.js");
const jsFixture = path.join(repoRoot, "tests/mcp/fixtures/lsp/sample.js");
const externalWorkspace = path.join(repoRoot, "tests/mcp/fixtures/lsp/external-workspace"), externalFixture = path.join(externalWorkspace, "src/sample.js");
const allowOverride = { CODEXY_LSP_ALLOW_COMMAND_OVERRIDE: "1" }, fakeEnv = (env = {}) => ({ ...allowOverride, ...env });
async function toolPayload(client, name, args) { return assertStructuredToolResult(assert, await client.callTool(name, args)); }
test("lsp_status reports availability and install hints for a JavaScript path", async () => {
  await withLspClient(assert, lspServer, repoRoot, async (client) => {
    const response = await client.callTool("lsp_status", { path: jsFixture });
    const status = assertStructuredToolResult(assert, response);
    assert.equal(status.path, jsFixture); assert.equal(status.language, "javascript");
    assert.equal(status.server.id, "typescript"); assert.match(status.server.executable, /typescript-language-server/);
    assert.equal(typeof status.available, "boolean");
    assert.ok(Array.isArray(status.installHints)); assert.ok(status.installHints.length > 0);
  });
});
test("full LSP operation tools are registered", async () => {
  await withLspClient(assert, lspServer, repoRoot, async (client) => {
    const tools = await client.listTools();
    const names = tools.map((tool) => tool.name);
    const statusTool = tools.find((tool) => tool.name === "lsp_status"), symbolTool = tools.find((tool) => tool.name === "lsp_document_symbols");
    for (const name of ["lsp_document_symbols", "lsp_definition", "lsp_references", "lsp_diagnostics"]) assert.ok(names.includes(name));
    assert.ok(statusTool.inputSchema.properties.root);
    assert.ok(symbolTool.inputSchema.properties.root);
  });
});
test("LSP operations return structured unavailable when the server executable is missing", async () => {
  await withLspClient(assert, lspServer, repoRoot, async (client) => {
    const payload = await toolPayload(client, "lsp_document_symbols", { path: jsFixture, server: { id: "missing-test-server", command: ["definitely-not-a-real-language-server", "--stdio"] } });
    assert.equal(payload.status, "unavailable");
    assert.equal(payload.server.id, "missing-test-server");
    assert.match(payload.reason, /not found|missing|unavailable/i);
    assert.ok(Array.isArray(payload.installHints));
  }, { env: allowOverride });
});
test("LSP operations return structured unavailable when a path command is not executable", async () => {
  const commandPath = path.join(os.tmpdir(), `codexy-non-executable-lsp-${process.pid}-${Date.now()}`);
  try {
    fs.writeFileSync(commandPath, "#!/bin/sh\nexit 0\n", { mode: 0o600 });
    await withLspClient(assert, lspServer, repoRoot, async (client) => {
      const payload = await toolPayload(client, "lsp_document_symbols", { path: jsFixture, server: { id: "non-executable-test-server", command: [commandPath, "--stdio"] } });
      assert.equal(payload.status, "unavailable");
      assert.equal(payload.server.id, "non-executable-test-server");
      assert.match(payload.reason, /not executable|permission|unavailable/i);
      assert.ok(Array.isArray(payload.installHints));
    }, { env: allowOverride });
  } finally {
    fs.rmSync(commandPath, { force: true });
  }
});
test("LSP operations return structured errors when an executable path cannot spawn", async () => {
  const commandPath = fs.mkdtempSync(path.join(os.tmpdir(), `codexy-unspawnable-lsp-${process.pid}-`));
  try {
    await withLspClient(assert, lspServer, repoRoot, async (client) => {
      const payload = await toolPayload(client, "lsp_document_symbols", { path: jsFixture, server: { id: "unspawnable-test-server", command: [commandPath, "--stdio"] } });
      assert.equal(payload.status, "error");
      assert.equal(payload.server.id, "unspawnable-test-server");
      assert.match(payload.reason, /spawn|EACCES|permission|directory/i);
      assert.ok(Array.isArray(payload.installHints));
    }, { env: allowOverride });
  } finally {
    fs.rmSync(commandPath, { recursive: true, force: true });
  }
});
test("LSP operations reject caller command overrides by default", async () => {
  const markerPath = path.join(os.tmpdir(), `codexy-lsp-command-marker-${process.pid}-${Date.now()}`);
  try {
    await withLspClient(assert, lspServer, repoRoot, async (client) => {
      const payload = await toolPayload(client, "lsp_document_symbols", { path: jsFixture, server: { id: "typescript", command: ["/bin/sh", "-c", `touch ${markerPath}`] } });
      assert.equal(payload.status, "unavailable");
      assert.match(payload.reason, /command overrides require CODEXY_LSP_ALLOW_COMMAND_OVERRIDE=1/);
      assert.equal(fs.existsSync(markerPath), false);
    });
  } finally {
    fs.rmSync(markerPath, { force: true });
  }
});
test("LSP operations initialize servers from the target file workspace root", async () => {
  await withFakeLspCapture("codexy-fake-lsp", async (capturePath) => {
    await withLspClient(assert, lspServer, repoRoot, async (client) => {
      const payload = await toolPayload(client, "lsp_document_symbols", { path: externalFixture, server: { id: "fake-lsp", command: fakeLspCommand() } });
      assert.equal(payload.status, "ok");
      assert.equal(payload.path, externalFixture);
    }, { env: fakeEnv({ CODEXY_FAKE_LSP_CAPTURE: capturePath }) });
    const capture = readCapture(capturePath);
    assert.equal(capture.cwd, externalWorkspace);
    assert.equal(capture.rootUri, pathToFileURL(externalWorkspace).href);
  });
});
test("LSP operations keep a markerless target file directory as the workspace root", async () => {
  await withMarkerlessWorkspace("codexy-markerless-lsp", "sample.js", async ({ workspaceRoot, filePath }) => {
    await withFakeLspCapture("codexy-fake-lsp-markerless", async (capturePath) => {
      await withLspClient(assert, lspServer, repoRoot, async (client) => {
        const payload = await toolPayload(client, "lsp_document_symbols", { path: filePath, server: { id: "fake-lsp", command: fakeLspCommand() } });
        assert.equal(payload.status, "ok");
        assert.equal(payload.path, filePath);
      }, { env: fakeEnv({ CODEXY_FAKE_LSP_CAPTURE: capturePath }) });
      const capture = readCapture(capturePath);
      assert.equal(fs.realpathSync(capture.cwd), fs.realpathSync(workspaceRoot));
      assert.equal(capture.rootUri, pathToFileURL(workspaceRoot).href);
    });
  });
});
test("LSP operations resolve relative paths against caller root", async () => {
  await withFakeLspCapture("codexy-fake-lsp-root", async (capturePath) => {
    await withLspClient(assert, lspServer, repoRoot, async (client) => {
      const status = await toolPayload(client, "lsp_status", { path: "src/sample.js", root: externalWorkspace });
      assert.equal(status.path, externalFixture);
      const payload = await toolPayload(client, "lsp_document_symbols", {
        path: "src/sample.js",
        root: externalWorkspace,
        server: { id: "fake-lsp", command: fakeLspCommand() },
      });
      const externalUri = pathToFileURL(externalFixture).href;
      assert.equal(payload.status, "ok");
      assert.equal(payload.path, externalFixture);
      const capture = readCapture(capturePath);
      assert.equal(capture.cwd, externalWorkspace);
      assert.equal(capture.rootUri, pathToFileURL(externalWorkspace).href);
      assert.equal(capture.openedUri, externalUri);
      assert.equal(capture.requestUri, externalUri);
    }, { cwd: pluginRoot, env: fakeEnv({ CODEXY_FAKE_LSP_CAPTURE: capturePath }) });
  });
});
test("LSP operations honor caller workspaceRoot inside markerless workspaces", async () => {
  await withMarkerlessWorkspace("codexy-markerless-relative-lsp", "src/sample.js", async ({ workspaceRoot, filePath }) => {
    for (const testCase of [{ cwd: pluginRoot, workspaceRoot, label: "absolute workspaceRoot from plugin cwd" }, { cwd: workspaceRoot, workspaceRoot: ".", label: "relative workspaceRoot from workspace cwd" }]) {
      await withFakeLspCapture("codexy-fake-lsp-markerless-relative", async (capturePath) => {
        await withLspClient(assert, lspServer, repoRoot, async (client) => {
          const payload = await toolPayload(client, "lsp_document_symbols", {
            path: "src/sample.js", workspaceRoot: testCase.workspaceRoot, server: { id: "fake-lsp", command: fakeLspCommand() },
          });
          assert.equal(payload.status, "ok", testCase.label);
          assert.equal(fs.realpathSync(payload.path), fs.realpathSync(filePath), testCase.label);
        }, { cwd: testCase.cwd, env: fakeEnv({ CODEXY_FAKE_LSP_CAPTURE: capturePath }) });
        const capture = readCapture(capturePath);
        assert.equal(fs.realpathSync(capture.cwd), fs.realpathSync(workspaceRoot), testCase.label);
        assert.equal(fs.realpathSync(fileURLToPath(capture.rootUri)), fs.realpathSync(workspaceRoot), testCase.label);
      });
    }
  });
});
test("relative override commands resolve from caller workspaceRoot and spawn via the resolved absolute path", async () => {
  await withWorkspaceRelativeFakeLsp("codexy-relative-command-lsp", async ({ workspaceRoot, filePath, relativeCommand, resolvedCommand }) => {
    await withFakeLspCapture("codexy-fake-lsp-relative-command", async (capturePath) => {
      await withLspClient(assert, lspServer, repoRoot, async (client) => {
        const status = await toolPayload(client, "lsp_status", {
          path: "src/sample.js", workspaceRoot, server: { id: "fake-lsp", command: [relativeCommand] },
        });
        assert.equal(status.available, true);
        assert.equal(fs.realpathSync(status.server.resolvedExecutable), fs.realpathSync(resolvedCommand));
        const payload = await toolPayload(client, "lsp_document_symbols", {
          path: "src/sample.js", workspaceRoot, server: { id: "fake-lsp", command: [relativeCommand] },
        });
        assert.equal(payload.status, "ok");
        assert.equal(payload.path, filePath);
      }, { cwd: pluginRoot, env: fakeEnv({ CODEXY_FAKE_LSP_CAPTURE: capturePath }) });
      assert.equal(fs.realpathSync(readCapture(capturePath).cwd), fs.realpathSync(workspaceRoot));
    });
  });
});
test("LSP tools reject relative paths without an explicit caller root", async () => {
  await withLspClient(assert, lspServer, repoRoot, async (client) => {
    const response = await client.callTool("lsp_status", { path: "src/sample.js" });
    assert.equal(response.error?.code, -32000);
    assert.match(response.error?.message || "", /root.*required.*relative path/i);
  }, { cwd: pluginRoot });
});
test("LSP operations answer server-to-client requests before document symbols", async () => {
  await withFakeLspCapture("codexy-fake-lsp-server-request", async (capturePath) => {
    await withLspClient(assert, lspServer, repoRoot, async (client) => {
      const payload = await toolPayload(client, "lsp_document_symbols", {
        path: externalFixture,
        server: { id: "fake-lsp", command: fakeLspCommand() },
      });
      assert.equal(payload.status, "ok");
      assert.equal(payload.path, externalFixture);
      const capture = readCapture(capturePath);
      assert.equal(capture.serverRequestResponseId, 1000);
      assert.deepEqual(capture.serverRequestResponseResult, [null]);
    }, { env: fakeEnv({ CODEXY_FAKE_LSP_CAPTURE: capturePath, CODEXY_FAKE_LSP_REQUIRE_CLIENT_RESPONSE: "1" }) });
  });
});
test("LSP operations do not confuse a colliding server request id with the pending client request", async () => {
  await withFakeLspCapture("codexy-fake-lsp-colliding-server-request", async (capturePath) => {
    await withLspClient(assert, lspServer, repoRoot, async (client) => {
      const payload = await toolPayload(client, "lsp_document_symbols", {
        path: externalFixture,
        server: { id: "fake-lsp", command: fakeLspCommand() },
      });
      assert.equal(payload.status, "ok");
      assert.equal(payload.path, externalFixture);
      assert.deepEqual(payload.result, []);
      const capture = readCapture(capturePath);
      assert.equal(capture.serverRequestResponseId, 2);
      assert.deepEqual(capture.serverRequestResponseResult, [null]);
    }, { env: fakeEnv({ CODEXY_FAKE_LSP_CAPTURE: capturePath, CODEXY_FAKE_LSP_REQUIRE_CLIENT_RESPONSE: "1", CODEXY_FAKE_LSP_SERVER_REQUEST_ID: "match-client-request" }) });
  });
});
test("lsp_diagnostics uses pull diagnostics when the server advertises diagnosticProvider", async () => {
  await withFakeLspCapture("codexy-fake-lsp-pull-diagnostics", async (capturePath) => {
    await withLspClient(assert, lspServer, repoRoot, async (client) => {
      const payload = await toolPayload(client, "lsp_diagnostics", {
        path: externalFixture,
        server: { id: "fake-lsp", command: fakeLspCommand() },
      });
      assert.equal(payload.status, "ok");
      assert.deepEqual(payload.result, []);
      assert.equal(payload.diagnostics[0].uri, pathToFileURL(externalFixture).href);
    }, { env: fakeEnv({ CODEXY_FAKE_LSP_CAPTURE: capturePath, CODEXY_FAKE_LSP_PULL_DIAGNOSTICS: "1" }) });
    const capture = readCapture(capturePath);
    assert.ok(capture.requestMethods.includes("textDocument/diagnostic"));
  });
});
test("lsp_diagnostics falls back to publish diagnostics for push-only servers", async () => {
  await withFakeLspCapture("codexy-fake-lsp-push-diagnostics", async (capturePath) => {
    await withLspClient(assert, lspServer, repoRoot, async (client) => {
      const payload = await toolPayload(client, "lsp_diagnostics", {
        path: externalFixture,
        server: { id: "fake-lsp", command: fakeLspCommand() },
      });
      assert.equal(payload.status, "ok");
      assert.equal(payload.result, null);
      assert.equal(payload.diagnostics[0].diagnostics[0].message, "push-only diagnostic");
    }, { env: fakeEnv({ CODEXY_FAKE_LSP_CAPTURE: capturePath, CODEXY_FAKE_LSP_PUSH_DIAGNOSTICS_ON_OPEN: "1" }) });
    const capture = readCapture(capturePath);
    assert.ok(!(capture.requestMethods || []).includes("textDocument/diagnostic"));
  });
});
test("LSP operations return structured errors when server stdin closes early", async () => {
  await withLspClient(assert, lspServer, repoRoot, async (client) => {
    const payload = await toolPayload(client, "lsp_document_symbols", {
      path: externalFixture,
      server: { id: "fake-lsp", command: fakeLspCommand() },
    });
    assert.equal(payload.status, "error");
    assert.match(payload.reason, /stdin unavailable|exited before response|server closed stdin/i);
  }, { env: fakeEnv({ CODEXY_FAKE_LSP_EXIT_EARLY: "1" }) });
});
