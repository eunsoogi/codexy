"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("fs/promises");
const os = require("os");
const path = require("path");
const { createStdioClient, jsonTextContent } = require("./stdio-client");

const repoRoot = path.resolve(__dirname, "..", "..");
const codegraphServer = path.join(repoRoot, "plugins/codexy/mcp/codegraph/server.js");
const fixtureRoot = path.join(__dirname, "fixtures/codegraph");

async function withCodegraphClient(fn) {
  const client = createStdioClient(process.execPath, [codegraphServer], { cwd: repoRoot });
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

async function tool(client, name, args) {
  return assertStructuredToolResult(await client.callTool(name, args));
}

test("graph index includes imports, exports, and resolved/unresolved edges", async () => {
  await withCodegraphClient(async (client) => {
    const graph = await tool(client, "codegraph_index", { root: fixtureRoot, limit: 10 });

    assert.equal(graph.root, fixtureRoot);
    assert.ok(Array.isArray(graph.files));
    assert.ok(graph.files.some((file) => file.path === "entry.js" && file.imports.length === 2));
    assert.ok(graph.files.some((file) => file.path === "helper.js" && file.exports.includes("helper")));
    assert.ok(graph.edges.some((edge) => edge.from === "entry.js" && edge.to === "helper.js" && edge.resolved === true));
    assert.ok(graph.edges.some((edge) => edge.from === "entry.js" && edge.specifier === "./missing.js" && edge.resolved === false));
  });
});

test("reverse deps and bounded neighborhood tools are registered", async () => {
  await withCodegraphClient(async (client) => {
    const tools = await client.listTools();
    const names = tools.map((tool) => tool.name);

    assert.ok(names.includes("codegraph_reverse_deps"));
    assert.ok(names.includes("codegraph_neighborhood"));

    const reverse = await tool(client, "codegraph_reverse_deps", { root: fixtureRoot, path: "helper.js", limit: 5 });
    assert.deepEqual(reverse.dependents.map((entry) => entry.path), ["entry.js"]);

    const neighborhood = await tool(client, "codegraph_neighborhood", { root: fixtureRoot, path: "entry.js", depth: 1, limit: 1 });
    assert.equal(neighborhood.nodes.length, 1);
    assert.equal(neighborhood.limit, 1);
    assert.equal(typeof neighborhood.truncated, "boolean");
    const nodePaths = new Set(neighborhood.nodes.map((node) => node.path));
    const orphanEdges = neighborhood.edges.filter((edge) => !nodePaths.has(edge.from) || !nodePaths.has(edge.to));
    assert.deepEqual(orphanEdges, []);
  });
});

test("JS-family relative specifiers resolve to TS and TSX siblings", async () => {
  const emittedSpecifierRoot = await fs.mkdtemp(path.join(os.tmpdir(), "codegraph-js-specifier-"));
  await fs.writeFile(path.join(emittedSpecifierRoot, "bar.ts"), 'import { foo } from "./foo.js";\nexport const bar = () => foo();\n', "utf8");
  await fs.writeFile(path.join(emittedSpecifierRoot, "foo.ts"), "export const foo = () => 1;\n", "utf8");
  await fs.writeFile(path.join(emittedSpecifierRoot, "view.tsx"), 'import { Widget } from "./widget.js";\nexport const View = () => Widget();\n', "utf8");
  await fs.writeFile(path.join(emittedSpecifierRoot, "widget.tsx"), "export const Widget = () => null;\n", "utf8");

  try {
    await withCodegraphClient(async (client) => {
      const graph = await tool(client, "codegraph_index", { root: emittedSpecifierRoot, limit: 10 });
      assert.ok(graph.edges.some((edge) => edge.from === "bar.ts" && edge.to === "foo.ts" && edge.specifier === "./foo.js" && edge.resolved === true));
      assert.ok(graph.edges.some((edge) => edge.from === "view.tsx" && edge.to === "widget.tsx" && edge.specifier === "./widget.js" && edge.resolved === true));
      assert.deepEqual((await tool(client, "codegraph_reverse_deps", { root: emittedSpecifierRoot, path: "foo.ts", limit: 5 })).dependents, [{ path: "bar.ts", specifier: "./foo.js" }]);
      assert.ok((await tool(client, "codegraph_neighborhood", { root: emittedSpecifierRoot, path: "bar.ts", depth: 1, limit: 5 })).nodes.some((node) => node.path === "foo.ts"));
    });
  } finally {
    await fs.rm(emittedSpecifierRoot, { recursive: true, force: true });
  }
});

test("directory index imports resolve for graph, reverse deps, and neighborhood", async () => {
  const indexFixtureRoot = await fs.mkdtemp(path.join(os.tmpdir(), "codegraph-index-import-"));
  await fs.mkdir(path.join(indexFixtureRoot, "feature"));
  await fs.writeFile(path.join(indexFixtureRoot, "index-entry.js"), 'import { feature } from "./feature";\nexport function runFeature() {\n  return feature();\n}\n', "utf8");
  await fs.writeFile(path.join(indexFixtureRoot, "feature/index.ts"), "export function feature() {\n  return 2;\n}\n", "utf8");

  try {
    await withCodegraphClient(async (client) => {
      const graph = await tool(client, "codegraph_index", { root: indexFixtureRoot, limit: 10 });

      assert.ok(graph.edges.some((edge) => edge.from === "index-entry.js" && edge.to === "feature/index.ts" && edge.specifier === "./feature" && edge.resolved === true));

      const reverse = await tool(client, "codegraph_reverse_deps", { root: indexFixtureRoot, path: "feature/index.ts", limit: 5 });
      assert.deepEqual(reverse.dependents, [{ path: "index-entry.js", specifier: "./feature" }]);

      const neighborhood = await tool(client, "codegraph_neighborhood", { root: indexFixtureRoot, path: "index-entry.js", depth: 1, limit: 5 });
      assert.ok(neighborhood.nodes.some((node) => node.path === "feature/index.ts"));
    });
  } finally {
    await fs.rm(indexFixtureRoot, { recursive: true, force: true });
  }
});

test("re-export specifiers create dependency edges across graph tools", async () => {
  const reexportFixtureRoot = await fs.mkdtemp(path.join(os.tmpdir(), "codegraph-reexport-"));
  await fs.writeFile(path.join(reexportFixtureRoot, "mod.js"), "export const leaf = 1;\n", "utf8");
  await fs.writeFile(path.join(reexportFixtureRoot, "star.js"), 'export * from "./mod.js";\n', "utf8");
  await fs.writeFile(path.join(reexportFixtureRoot, "named.js"), 'export { leaf as renamedLeaf } from "./mod.js";\n', "utf8");
  await fs.writeFile(path.join(reexportFixtureRoot, "types.ts"), "export type Foo = { leaf: number };\n", "utf8");
  await fs.writeFile(path.join(reexportFixtureRoot, "typed.ts"), 'export type { Foo } from "./types";\n', "utf8");

  try {
    await withCodegraphClient(async (client) => {
      const graph = await tool(client, "codegraph_index", { root: reexportFixtureRoot, limit: 10 });

      assert.ok(graph.files.some((file) => file.path === "named.js" && file.exports.includes("renamedLeaf")));
      assert.ok(graph.edges.some((edge) => edge.from === "star.js" && edge.to === "mod.js" && edge.resolved === true));
      assert.ok(graph.edges.some((edge) => edge.from === "named.js" && edge.to === "mod.js" && edge.resolved === true));
      assert.ok(graph.files.some((file) => file.path === "typed.ts" && file.exports.includes("Foo")));
      assert.ok(graph.edges.some((edge) => edge.from === "typed.ts" && edge.to === "types.ts" && edge.resolved === true));

      const reverse = await tool(client, "codegraph_reverse_deps", { root: reexportFixtureRoot, path: "mod.js", limit: 5 });
      assert.deepEqual(reverse.dependents.map((entry) => entry.path), ["named.js", "star.js"]);
      const typedReverse = await tool(client, "codegraph_reverse_deps", { root: reexportFixtureRoot, path: "types.ts", limit: 5 });
      assert.deepEqual(typedReverse.dependents, [{ path: "typed.ts", specifier: "./types" }]);

      const neighborhood = await tool(client, "codegraph_neighborhood", { root: reexportFixtureRoot, path: "named.js", depth: 1, limit: 5 });
      assert.ok(neighborhood.nodes.some((node) => node.path === "mod.js"));
      const typedNeighborhood = await tool(client, "codegraph_neighborhood", { root: reexportFixtureRoot, path: "typed.ts", depth: 1, limit: 5 });
      assert.ok(typedNeighborhood.nodes.some((node) => node.path === "types.ts"));
    });
  } finally {
    await fs.rm(reexportFixtureRoot, { recursive: true, force: true });
  }
});

test("oversized graph output reports limit and truncation metadata", async () => {
  await withCodegraphClient(async (client) => {
    const response = await client.callTool("codegraph_index", { root: repoRoot, limit: 1 });
    const graph = assertStructuredToolResult(response);

    assert.equal(graph.limit, 1);
    assert.equal(graph.truncated, true);
    assert.ok(graph.totalFiles > graph.files.length);
    assert.ok(graph.metadata);
    assert.equal(graph.metadata.truncated, true);
  });
});

test("graph index keeps existing imports resolved outside the bounded file list", async () => {
  await withCodegraphClient(async (client) => {
    const graph = await tool(client, "codegraph_index", { root: fixtureRoot, limit: 1 });

    assert.deepEqual(graph.files.map((file) => file.path), ["entry.js"]);
    assert.ok(graph.edges.some((edge) => edge.from === "entry.js" && edge.to === "helper.js" && edge.specifier === "./helper.js" && edge.resolved === true));
  });
});

test("commented imports do not create graph dependencies", async () => {
  const commentedRoot = await fs.mkdtemp(path.join(os.tmpdir(), "codegraph-commented-imports-"));
  await fs.writeFile(path.join(commentedRoot, "entry.js"), [
    'import { live } from "./live.js";',
    '// import { line } from "./line-commented.js";',
    '/* import { block } from "./block-commented.js"; */',
    "export const value = live;",
    "",
  ].join("\n"), "utf8");
  await fs.writeFile(path.join(commentedRoot, "live.js"), "export const live = 1;\n", "utf8");
  await fs.writeFile(path.join(commentedRoot, "regex.js"), 'const marker = /\\/\\//; const regexLive = require("./regex-live.js");\nexport { regexLive };\n', "utf8");
  await fs.writeFile(path.join(commentedRoot, "regex-live.js"), "exports.regexLive = 4;\n", "utf8");
  await fs.writeFile(path.join(commentedRoot, "line-commented.js"), "export const line = 2;\n", "utf8");
  await fs.writeFile(path.join(commentedRoot, "block-commented.js"), "export const block = 3;\n", "utf8");

  try {
    await withCodegraphClient(async (client) => {
      const graph = await tool(client, "codegraph_index", { root: commentedRoot, limit: 10 });
      assert.deepEqual(graph.edges.map((edge) => edge.to), ["live.js", "regex-live.js"]);
      assert.deepEqual((await tool(client, "codegraph_reverse_deps", { root: commentedRoot, path: "line-commented.js", limit: 5 })).dependents, []);
      assert.deepEqual((await tool(client, "codegraph_neighborhood", { root: commentedRoot, path: "entry.js", depth: 1, limit: 10 })).nodes.map((node) => node.path), ["entry.js", "live.js"]);
    });
  } finally {
    await fs.rm(commentedRoot, { recursive: true, force: true });
  }
});

test("isolated neighborhoods do not report truncation because unrelated repo files exist", async () => {
  await withCodegraphClient(async (client) => {
    const neighborhood = await tool(client, "codegraph_neighborhood", { root: fixtureRoot, path: "helper.js", depth: 1, limit: 1 });

    assert.deepEqual(neighborhood.nodes, [{ path: "helper.js" }]);
    assert.deepEqual(neighborhood.edges, []);
    assert.equal(neighborhood.truncated, false);
  });
});

test("graph index excludes git-ignored local state files", async () => {
  const sourceRelativePath = "tests/mcp/fixtures/codegraph/imports-ignored-local-state.js";
  const sourceAbsolutePath = path.join(repoRoot, sourceRelativePath);
  const ignoredRelativePath = ".omo/ulw-loop/full-lsp-codegraph/evidence/ignored-local-state.js";
  const ignoredAbsolutePath = path.join(repoRoot, ignoredRelativePath);
  const ignoredSpecifier = path.posix.relative(path.posix.dirname(sourceRelativePath), ignoredRelativePath);
  await fs.writeFile(sourceAbsolutePath, `import "${ignoredSpecifier}";\n`, "utf8");
  await fs.mkdir(path.dirname(ignoredAbsolutePath), { recursive: true });
  await fs.writeFile(ignoredAbsolutePath, "export const ignoredLocalState = true;\n", "utf8");

  try {
    await withCodegraphClient(async (client) => {
      const response = await client.callTool("codegraph_index", { root: repoRoot, limit: Number.MAX_SAFE_INTEGER });
      const graph = assertStructuredToolResult(response);
      const indexedFiles = graph.files.map((file) => file.path);

      assert.ok(!indexedFiles.includes(ignoredRelativePath), `${ignoredRelativePath} should not be indexed`);
      assert.ok(!graph.edges.some((edge) => edge.from === ignoredRelativePath || edge.to === ignoredRelativePath));
      assert.ok(graph.edges.some((edge) => edge.from === sourceRelativePath && edge.to === ignoredSpecifier && edge.resolved === false));
      assert.deepEqual((await tool(client, "codegraph_reverse_deps", { root: repoRoot, path: ignoredRelativePath, limit: 5 })).dependents, []);
      const neighborhood = await tool(client, "codegraph_neighborhood", { root: repoRoot, path: sourceRelativePath, depth: 1, limit: 5 });
      assert.deepEqual(neighborhood.nodes, [{ path: sourceRelativePath }]);
      assert.deepEqual(neighborhood.edges, []);
    });
  } finally {
    await fs.rm(sourceAbsolutePath, { force: true });
    await fs.rm(ignoredAbsolutePath, { force: true });
  }
});
