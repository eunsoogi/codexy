"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const path = require("path");
const { createStdioClient, jsonTextContent, textContent } = require("./stdio-client");

const repoRoot = path.resolve(__dirname, "..", "..");
const lspServer = path.join(repoRoot, "plugins/codexy/mcp/lsp/server.js");
const codegraphServer = path.join(repoRoot, "plugins/codexy/mcp/codegraph/server.js");
const codegraphFixture = path.join(__dirname, "fixtures/codegraph");

async function withClient(serverPath, fn) {
  const client = createStdioClient(process.execPath, [serverPath], { cwd: repoRoot });
  try {
    await client.initialize();
    return await fn(client);
  } finally {
    await client.close();
  }
}

test("lsp compatibility tools are still registered", async () => {
  await withClient(lspServer, async (client) => {
    const tools = await client.listTools();
    const names = tools.map((tool) => tool.name);

    assert.ok(names.includes("lsp_list_servers"));
    assert.ok(names.includes("lsp_for_path"));
  });
});

test("lsp_for_path still maps JavaScript files to the TypeScript server", async () => {
  await withClient(lspServer, async (client) => {
    const response = await client.callTool("lsp_for_path", { path: "tests/mcp/fixtures/lsp/sample.js" });
    const matches = jsonTextContent(response);

    assert.ok(matches.some((server) => server.id === "typescript"));
  });
});

test("codegraph compatibility tools are still registered", async () => {
  await withClient(codegraphServer, async (client) => {
    const tools = await client.listTools();
    const names = tools.map((tool) => tool.name);

    assert.ok(names.includes("codegraph_overview"));
    assert.ok(names.includes("codegraph_search"));
    assert.ok(names.includes("codegraph_neighbors"));
  });
});

test("codegraph_overview/search/neighbors keep their current baseline behavior", async () => {
  await withClient(codegraphServer, async (client) => {
    const overview = jsonTextContent(
      await client.callTool("codegraph_overview", { root: codegraphFixture, limit: 10 })
    );
    assert.equal(overview.fileCount, 2);
    assert.ok(overview.files.includes("entry.js"));
    assert.ok(overview.importEdges.some((edge) => edge.file === "entry.js" && edge.text.includes("./helper.js")));

    const search = textContent(
      await client.callTool("codegraph_search", { root: codegraphFixture, query: "helper", limit: 5 })
    );
    assert.match(search, /helper\.js|entry\.js/);

    const neighbors = jsonTextContent(
      await client.callTool("codegraph_neighbors", { root: codegraphFixture, path: "entry.js" })
    );
    assert.ok(neighbors.some((edge) => edge.text.includes("./missing.js")));
  });
});
