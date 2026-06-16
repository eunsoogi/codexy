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

test("TypeScript type declarations are included in graph exports", async () => {
  const typeExportRoot = await fs.mkdtemp(path.join(os.tmpdir(), "codegraph-ts-exports-"));
  await fs.writeFile(path.join(typeExportRoot, "types.ts"), [
    "export interface Props {}",
    "export type Alias = { value: string };",
    "export enum Mode { Read }",
    "",
  ].join("\n"), "utf8");

  try {
    await withCodegraphClient(async (client) => {
      const graph = await tool(client, "codegraph_index", { root: typeExportRoot, limit: 5 });
      const entry = graph.files.find((file) => file.path === "types.ts");
      assert.deepEqual(entry.exports.sort(), ["Alias", "Mode", "Props"]);
    });
  } finally {
    await fs.rm(typeExportRoot, { recursive: true, force: true });
  }
});

test("absolute root-contained paths match reverse deps and neighborhoods", async () => {
  const absolutePathRoot = await fs.mkdtemp(path.join(os.tmpdir(), "codegraph-absolute-paths-"));
  await fs.writeFile(path.join(absolutePathRoot, "entry.ts"), 'import { leaf } from "./leaf";\nexport const entry = leaf;\n', "utf8");
  await fs.writeFile(path.join(absolutePathRoot, "leaf.ts"), "export const leaf = 1;\n", "utf8");

  try {
    await withCodegraphClient(async (client) => {
      const reverse = await tool(client, "codegraph_reverse_deps", { root: absolutePathRoot, path: path.join(absolutePathRoot, "leaf.ts"), limit: 5 });
      assert.deepEqual(reverse.dependents, [{ path: "entry.ts", specifier: "./leaf" }]);

      const neighborhood = await tool(client, "codegraph_neighborhood", { root: absolutePathRoot, path: path.join(absolutePathRoot, "entry.ts"), depth: 1, limit: 5 });
      assert.deepEqual(neighborhood.nodes.map((node) => node.path), ["entry.ts", "leaf.ts"]);
      assert.deepEqual(neighborhood.edges.map((edge) => [edge.from, edge.to]), [["entry.ts", "leaf.ts"]]);
    });
  } finally {
    await fs.rm(absolutePathRoot, { recursive: true, force: true });
  }
});

test("advertised non-JS files create graph edges across index, reverse deps, and neighborhoods", async () => {
  const nonJsRoot = await fs.mkdtemp(path.join(os.tmpdir(), "codegraph-non-js-"));
  await fs.mkdir(path.join(nonJsRoot, "local"), { recursive: true });
  await Promise.all([
    fs.mkdir(path.join(nonJsRoot, "pkg/sub"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "pkg/sub/app.py"), "from ..util import helper\n", "utf8")),
    fs.mkdir(path.join(nonJsRoot, "pkg/sub"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "pkg/sub/absolute_app.py"), "from pkg.util import helper\n", "utf8")),
    fs.writeFile(path.join(nonJsRoot, "app.py"), "from .util import helper\nfrom . import sibling_py\nfrom . import package_py\nimport localpkg.mod\n\ndef run():\n    return helper()\n", "utf8"),
    fs.writeFile(path.join(nonJsRoot, "text.py"), '# import fake\ntext = "from .fake import thing"\n', "utf8"),
    fs.writeFile(path.join(nonJsRoot, "fake.py"), "def thing():\n    return 0\n", "utf8"),
    fs.writeFile(path.join(nonJsRoot, "helper.py"), "value = 1\n", "utf8"),
    fs.writeFile(path.join(nonJsRoot, "sibling_py.py"), "value = 1\n", "utf8"),
    fs.mkdir(path.join(nonJsRoot, "pkg"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "pkg/util.py"), "def helper():\n    return 2\n", "utf8")),
    fs.mkdir(path.join(nonJsRoot, "pkg/sub/pkg"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "pkg/sub/pkg/util.py"), "def helper():\n    return 3\n", "utf8")),
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
    fs.mkdir(path.join(nonJsRoot, "src"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "src/parent.rs"), "mod child;\n", "utf8")),
    fs.mkdir(path.join(nonJsRoot, "src"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "src/child.rs"), "pub fn wrong_child() {}\n", "utf8")),
    fs.mkdir(path.join(nonJsRoot, "src/parent"), { recursive: true }).then(() => fs.writeFile(path.join(nonJsRoot, "src/parent/child.rs"), "pub fn child() {}\n", "utf8")),
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
      const graph = await tool(client, "codegraph_index", { root: nonJsRoot, limit: 50 });
      const expectedEdges = [["app.py", "util.py"], ["app.py", "sibling_py.py"], ["app.py", "package_py/__init__.py"], ["pkg/sub/app.py", "pkg/util.py"], ["pkg/sub/absolute_app.py", "pkg/util.py"], ["app.py", "localpkg/mod.py"], ["main.go", "pkg.go"], ["lib.rs", "sibling.rs"], ["lib.rs", "support/thing.rs"], ["src/foo/bar.rs", "src/support/thing.rs"], ["src/parent.rs", "src/parent/child.rs"], ["module_root.rs", "module_dir/mod.rs"], ["app.rb", "worker.rb"], ["app.rb", "job.rb"], ["local/Main.java", "local/Helper.java"], ["local/Main.kt", "local/HelperKt.kt"]];
      assert.deepEqual(expectedEdges.filter(([from, to]) => !graph.edges.some((edge) => edge.from === from && edge.to === to && edge.resolved)), []);
      assert.ok(!graph.edges.some((edge) => edge.from === "main.go" && edge.to === "fmt.go"));
      assert.ok(!graph.edges.some((edge) => edge.from === "app.py" && edge.to === "helper.py"));
      assert.ok(!graph.edges.some((edge) => edge.from === "pkg/sub/absolute_app.py" && edge.to === "pkg/sub/pkg/util.py"));
      assert.ok(!graph.edges.some((edge) => edge.from === "src/parent.rs" && edge.to === "src/child.rs"));
      assert.ok(!graph.edges.some((edge) => edge.from === "text.py" && edge.to === "fake.py"));
      assert.deepEqual((await tool(client, "codegraph_reverse_deps", { root: nonJsRoot, path: "util.py", limit: 5 })).dependents, [{ path: "app.py", specifier: "./util" }]);
      assert.ok((await tool(client, "codegraph_neighborhood", { root: nonJsRoot, path: "lib.rs", depth: 1, limit: 5 })).nodes.some((node) => node.path === "support/thing.rs"));
    });
  } finally {
    await fs.rm(nonJsRoot, { recursive: true, force: true });
  }
});

test("extensionless non-JS imports prefer the importing language over JS decoys", async () => {
  const languagePreferenceRoot = await fs.mkdtemp(path.join(os.tmpdir(), "codegraph-language-preference-"));
  await Promise.all([
    fs.writeFile(path.join(languagePreferenceRoot, "app.py"), "from .util import helper\n", "utf8"),
    fs.writeFile(path.join(languagePreferenceRoot, "util.py"), "def helper():\n    return 1\n", "utf8"),
    fs.writeFile(path.join(languagePreferenceRoot, "util.js"), "export const helper = 2;\n", "utf8"),
    fs.writeFile(path.join(languagePreferenceRoot, "runner.rb"), 'require_relative "worker"\n', "utf8"),
    fs.writeFile(path.join(languagePreferenceRoot, "worker.rb"), "class Worker; end\n", "utf8"),
    fs.writeFile(path.join(languagePreferenceRoot, "worker.js"), "export class Worker {}\n", "utf8"),
  ]);

  try {
    await withCodegraphClient(async (client) => {
      const graph = await tool(client, "codegraph_index", { root: languagePreferenceRoot, limit: 10 });
      assert.ok(graph.edges.some((edge) => edge.from === "app.py" && edge.to === "util.py" && edge.resolved));
      assert.ok(graph.edges.some((edge) => edge.from === "runner.rb" && edge.to === "worker.rb" && edge.resolved));
      assert.ok(!graph.edges.some((edge) => edge.from === "app.py" && edge.to === "util.js"));
      assert.ok(!graph.edges.some((edge) => edge.from === "runner.rb" && edge.to === "worker.js"));
    });
  } finally {
    await fs.rm(languagePreferenceRoot, { recursive: true, force: true });
  }
});

test("Java and Kotlin package imports resolve from the package source root", async () => {
  const packageRoot = await fs.mkdtemp(path.join(os.tmpdir(), "codegraph-package-root-"));
  await Promise.all([
    fs.mkdir(path.join(packageRoot, "com/example/app/com/example/util"), { recursive: true }),
    fs.mkdir(path.join(packageRoot, "com/example/util"), { recursive: true }),
  ]);
  await Promise.all([
    fs.writeFile(path.join(packageRoot, "com/example/app/Main.java"), "package com.example.app;\nimport com.example.util.Helper;\nclass Main {}\n", "utf8"),
    fs.writeFile(path.join(packageRoot, "com/example/util/Helper.java"), "package com.example.util;\nclass Helper {}\n", "utf8"),
    fs.writeFile(path.join(packageRoot, "com/example/app/com/example/util/Helper.java"), "package wrong.root;\nclass Helper {}\n", "utf8"),
    fs.writeFile(path.join(packageRoot, "com/example/app/Main.kt"), "package com.example.app\nimport com.example.util.HelperKt\nfun main() {}\n", "utf8"),
    fs.writeFile(path.join(packageRoot, "com/example/util/HelperKt.kt"), "package com.example.util\nclass HelperKt\n", "utf8"),
    fs.writeFile(path.join(packageRoot, "com/example/app/com/example/util/HelperKt.kt"), "package wrong.root\nclass HelperKt\n", "utf8"),
  ]);

  try {
    await withCodegraphClient(async (client) => {
      const graph = await tool(client, "codegraph_index", { root: packageRoot, limit: 20 });
      assert.ok(graph.edges.some((edge) => edge.from === "com/example/app/Main.java" && edge.to === "com/example/util/Helper.java" && edge.resolved));
      assert.ok(graph.edges.some((edge) => edge.from === "com/example/app/Main.kt" && edge.to === "com/example/util/HelperKt.kt" && edge.resolved));
      assert.ok(!graph.edges.some((edge) => edge.from === "com/example/app/Main.java" && edge.to === "com/example/app/com/example/util/Helper.java"));
      assert.ok(!graph.edges.some((edge) => edge.from === "com/example/app/Main.kt" && edge.to === "com/example/app/com/example/util/HelperKt.kt"));
    });
  } finally {
    await fs.rm(packageRoot, { recursive: true, force: true });
  }
});

test("Java and Kotlin source-set imports resolve from detected source roots", async () => {
  const sourceSetRoot = await fs.mkdtemp(path.join(os.tmpdir(), "codegraph-source-set-root-"));
  const sourceSets = [
    ["src/main/java", "Main.java", "Helper.java", "class Main {}", "class Helper {}"],
    ["src/test/java", "MainTest.java", "HelperTest.java", "class MainTest {}", "class HelperTest {}"],
    ["src/main/kotlin", "Main.kt", "HelperKt.kt", "fun main() {}", "class HelperKt"],
    ["src/test/kotlin", "MainTest.kt", "HelperTestKt.kt", "fun testMain() {}", "class HelperTestKt"],
  ];

  for (const [sourceRoot, mainFile, helperFile, mainExport, helperExport] of sourceSets) {
    await Promise.all([
      fs.mkdir(path.join(sourceSetRoot, sourceRoot, "com/example/app"), { recursive: true }),
      fs.mkdir(path.join(sourceSetRoot, sourceRoot, "com/example/util"), { recursive: true }),
      fs.mkdir(path.join(sourceSetRoot, "com/example/util"), { recursive: true }),
    ]);
    const isKotlin = mainFile.endsWith(".kt");
    const packageLine = isKotlin ? "package com.example.app\n" : "package com.example.app;\n";
    const helperPackageLine = isKotlin ? "package com.example.util\n" : "package com.example.util;\n";
    const importLine = isKotlin ? `import com.example.util.${path.basename(helperFile, ".kt")}\n` : `import com.example.util.${path.basename(helperFile, ".java")};\n`;
    await Promise.all([
      fs.writeFile(path.join(sourceSetRoot, sourceRoot, "com/example/app", mainFile), `${packageLine}${importLine}${mainExport}\n`, "utf8"),
      fs.writeFile(path.join(sourceSetRoot, sourceRoot, "com/example/util", helperFile), `${helperPackageLine}${helperExport}\n`, "utf8"),
      fs.writeFile(path.join(sourceSetRoot, "com/example/util", helperFile), `${helperPackageLine}class WrongRoot {}\n`, "utf8"),
    ]);
  }

  try {
    await withCodegraphClient(async (client) => {
      const graph = await tool(client, "codegraph_index", { root: sourceSetRoot, limit: 50 });
      const expectedEdges = sourceSets.map(([sourceRoot, mainFile, helperFile]) => [
        `${sourceRoot}/com/example/app/${mainFile}`,
        `${sourceRoot}/com/example/util/${helperFile}`,
      ]);
      assert.deepEqual(expectedEdges.filter(([from, to]) => !graph.edges.some((edge) => edge.from === from && edge.to === to && edge.resolved)), []);
      assert.ok(!graph.edges.some((edge) => edge.to.startsWith("com/example/util/")));

      assert.deepEqual((await tool(client, "codegraph_reverse_deps", { root: sourceSetRoot, path: "src/main/java/com/example/util/Helper.java", limit: 5 })).dependents, [
        { path: "src/main/java/com/example/app/Main.java", specifier: "../util/Helper" },
      ]);
      const neighborhood = await tool(client, "codegraph_neighborhood", { root: sourceSetRoot, path: "src/main/kotlin/com/example/app/Main.kt", depth: 1, limit: 5 });
      assert.deepEqual(neighborhood.nodes.map((node) => node.path), [
        "src/main/kotlin/com/example/app/Main.kt",
        "src/main/kotlin/com/example/util/HelperKt.kt",
      ]);
    });
  } finally {
    await fs.rm(sourceSetRoot, { recursive: true, force: true });
  }
});
