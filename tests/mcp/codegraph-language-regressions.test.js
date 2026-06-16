"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("fs/promises");
const os = require("os");
const path = require("path");
const { createStdioClient, jsonTextContent } = require("./stdio-client");

const repoRoot = path.resolve(__dirname, "..", "..");
const codegraphServer = path.join(repoRoot, "plugins/codexy/mcp/codegraph/server.js");

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

test("export-like strings and templates do not create JS exports", async () => {
  const exportMaskRoot = await fs.mkdtemp(path.join(os.tmpdir(), "codegraph-export-mask-"));
  await fs.writeFile(path.join(exportMaskRoot, "entry.js"), [
    'const text = "export const fakeString = 1";',
    "const template = `export function fakeTemplate() {}`;",
    "export const real = 1;",
    "",
  ].join("\n"), "utf8");

  try {
    await withCodegraphClient(async (client) => {
      const graph = await tool(client, "codegraph_index", { root: exportMaskRoot, limit: 5 });
      const entry = graph.files.find((file) => file.path === "entry.js");
      assert.deepEqual(entry.exports, ["real"]);
    });
  } finally {
    await fs.rm(exportMaskRoot, { recursive: true, force: true });
  }
});

test("advertised non-JS files create graph edges across index, reverse deps, and neighborhoods", async () => {
  const nonJsRoot = await fs.mkdtemp(path.join(os.tmpdir(), "codegraph-non-js-"));
  await fs.mkdir(path.join(nonJsRoot, "local"), { recursive: true });
  await Promise.all([
    fs.mkdir(path.join(nonJsRoot, "pkg/sub"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "pkg/sub/app.py"), "from ..util import helper\n", "utf8")),
    fs.writeFile(path.join(nonJsRoot, "app.py"), "from .util import helper\nfrom . import sibling_py\nfrom . import package_py\nimport localpkg.mod\n\ndef run():\n    return helper()\n", "utf8"),
    fs.writeFile(path.join(nonJsRoot, "text.py"), '# import fake\ntext = "from .fake import thing"\n', "utf8"),
    fs.writeFile(path.join(nonJsRoot, "fake.py"), "def thing():\n    return 0\n", "utf8"),
    fs.writeFile(path.join(nonJsRoot, "helper.py"), "value = 1\n", "utf8"),
    fs.writeFile(path.join(nonJsRoot, "sibling_py.py"), "value = 1\n", "utf8"),
    fs.mkdir(path.join(nonJsRoot, "pkg"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "pkg/util.py"), "def helper():\n    return 2\n", "utf8")),
    fs.mkdir(path.join(nonJsRoot, "package_py"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "package_py/__init__.py"), "value = 1\n", "utf8")),
    fs.writeFile(path.join(nonJsRoot, "util.py"), "def helper():\n    return 1\n", "utf8"),
    fs.mkdir(path.join(nonJsRoot, "localpkg"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "localpkg/mod.py"), "value = 1\n", "utf8")),
    fs.writeFile(path.join(nonJsRoot, "main.go"), 'package main\n\nimport (\n  "fmt"\n  "./pkg"\n)\n', "utf8"),
    fs.writeFile(path.join(nonJsRoot, "fmt.go"), "package fmt\n", "utf8"),
    fs.writeFile(path.join(nonJsRoot, "pkg.go"), "package pkg\n", "utf8"),
    fs.writeFile(path.join(nonJsRoot, "lib.rs"), 'mod sibling;\nuse crate::support::thing;\n', "utf8"),
    fs.mkdir(path.join(nonJsRoot, "module_dir"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "module_dir/mod.rs"), "pub fn module_dir() {}\n", "utf8")),
    fs.writeFile(path.join(nonJsRoot, "module_root.rs"), "mod module_dir;\n", "utf8"),
    fs.mkdir(path.join(nonJsRoot, "src/foo"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "src/foo/bar.rs"), "use crate::support::thing;\n", "utf8")),
    fs.mkdir(path.join(nonJsRoot, "src/support"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "src/support/thing.rs"), "pub fn thing() {}\n", "utf8")),
    fs.writeFile(path.join(nonJsRoot, "sibling.rs"), "pub fn sibling() {}\n", "utf8"),
    fs.mkdir(path.join(nonJsRoot, "support"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "support/thing.rs"), "pub fn thing() {}\n", "utf8")),
    fs.writeFile(path.join(nonJsRoot, "app.rb"), 'require_relative "worker"\nrequire "./job"\n', "utf8"),
    fs.writeFile(path.join(nonJsRoot, "worker.rb"), "class Worker; end\n", "utf8"),
    fs.writeFile(path.join(nonJsRoot, "job.rb"), "class Job; end\n", "utf8"),
    fs.writeFile(path.join(nonJsRoot, "local/Main.java"), "package local;\nimport local.Helper;\nclass Main {}\n", "utf8"),
    fs.mkdir(path.join(nonJsRoot, "local"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "local/Helper.java"), "package local;\nclass Helper {}\n", "utf8")),
    fs.writeFile(path.join(nonJsRoot, "local/Main.kt"), "package local\nimport local.HelperKt\nfun main() {}\n", "utf8"),
    fs.mkdir(path.join(nonJsRoot, "local"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "local/HelperKt.kt"), "package local\nclass HelperKt\n", "utf8")),
  ]);

  try {
    await withCodegraphClient(async (client) => {
      const graph = await tool(client, "codegraph_index", { root: nonJsRoot, limit: 30 });
      for (const [from, to] of [["app.py", "util.py"], ["app.py", "sibling_py.py"], ["app.py", "package_py/__init__.py"], ["pkg/sub/app.py", "pkg/util.py"], ["app.py", "localpkg/mod.py"], ["main.go", "pkg.go"], ["lib.rs", "sibling.rs"], ["lib.rs", "support/thing.rs"], ["src/foo/bar.rs", "src/support/thing.rs"], ["module_root.rs", "module_dir/mod.rs"], ["app.rb", "worker.rb"], ["app.rb", "job.rb"], ["local/Main.java", "local/Helper.java"], ["local/Main.kt", "local/HelperKt.kt"]]) {
        assert.ok(graph.edges.some((edge) => edge.from === from && edge.to === to && edge.resolved), `${from} should resolve ${to}`);
      }
      assert.ok(!graph.edges.some((edge) => edge.from === "main.go" && edge.to === "fmt.go"));
      assert.ok(!graph.edges.some((edge) => edge.from === "app.py" && edge.to === "helper.py"));
      assert.ok(!graph.edges.some((edge) => edge.from === "text.py" && edge.to === "fake.py"));
      assert.deepEqual((await tool(client, "codegraph_reverse_deps", { root: nonJsRoot, path: "util.py", limit: 5 })).dependents, [{ path: "app.py", specifier: "./util" }]);
      assert.ok((await tool(client, "codegraph_neighborhood", { root: nonJsRoot, path: "lib.rs", depth: 1, limit: 5 })).nodes.some((node) => node.path === "support/thing.rs"));
    });
  } finally {
    await fs.rm(nonJsRoot, { recursive: true, force: true });
  }
});
