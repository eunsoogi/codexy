"use strict";

const fs = require("fs");
const path = require("path");
const { textResult } = require("../lib/stdio-mcp");
const { repoRoot, resultLimit } = require("./files");
const { buildGraph, neighborhood, reverseDeps } = require("./graph");
const { listCodeFiles, rgLines } = require("./ripgrep");

function importsFor(filePath) {
  if (!fs.existsSync(filePath)) return [];
  return fs
    .readFileSync(filePath, "utf8")
    .split(/\r?\n/)
    .map((line, index) => ({ line: index + 1, text: line.trim() }))
    .filter(({ text }) => /^(import\s|from\s+\S+\s+import|const\s+.*=\s*require\(|use\s+\S+::)/.test(text))
    .slice(0, 80);
}

const tools = [
  {
    name: "codegraph_overview",
    description: "Summarize code files and import edges for a repository root.",
    inputSchema: {
      type: "object",
      properties: {
        root: { type: "string", description: "Repository root. Defaults to the MCP process cwd." },
        limit: { type: "number", description: "Maximum code files to inspect." },
      },
    },
  },
  {
    name: "codegraph_search",
    description: "Search repository code with ripgrep and return path:line matches.",
    inputSchema: {
      type: "object",
      properties: {
        root: { type: "string" },
        query: { type: "string" },
        limit: { type: "number" },
      },
      required: ["query"],
    },
  },
  {
    name: "codegraph_neighbors",
    description: "Return import-like dependency lines for one source file.",
    inputSchema: {
      type: "object",
      properties: {
        root: { type: "string" },
        path: { type: "string" },
      },
      required: ["path"],
    },
  },
  {
    name: "codegraph_index",
    description: "Build a bounded code graph with import, export, edge, and truncation metadata.",
    inputSchema: {
      type: "object",
      properties: {
        root: { type: "string" },
        limit: { type: "number" },
      },
    },
  },
  {
    name: "codegraph_reverse_deps",
    description: "Return files that import a target path.",
    inputSchema: {
      type: "object",
      properties: {
        root: { type: "string" },
        path: { type: "string" },
        limit: { type: "number" },
      },
      required: ["path"],
    },
  },
  {
    name: "codegraph_neighborhood",
    description: "Return a bounded dependency neighborhood around one source file.",
    inputSchema: {
      type: "object",
      properties: {
        root: { type: "string" },
        path: { type: "string" },
        depth: { type: "number" },
        limit: { type: "number" },
      },
      required: ["path"],
    },
  },
];

async function callTool(name, args) {
  const root = repoRoot(args.root);
  if (name === "codegraph_overview") {
    const files = await listCodeFiles(root, args.limit || 400);
    const edges = files.flatMap((file) =>
      importsFor(path.join(root, file)).map((edge) => ({ file, ...edge }))
    );
    return textResult(JSON.stringify({ root, fileCount: files.length, files, importEdges: edges.slice(0, 300) }, null, 2));
  }
  if (name === "codegraph_search") {
    const output = await rgLines(["--hidden", "-n", "--glob", "!node_modules", "--glob", "!.git", "-e", args.query], root, resultLimit(args.limit));
    return textResult(output);
  }
  if (name === "codegraph_neighbors") {
    return textResult(JSON.stringify(importsFor(path.join(root, args.path)), null, 2));
  }
  if (name === "codegraph_index") {
    return textResult(JSON.stringify(buildGraph(root, args.limit), null, 2));
  }
  if (name === "codegraph_reverse_deps") {
    return textResult(JSON.stringify(reverseDeps(root, args.path, args.limit), null, 2));
  }
  if (name === "codegraph_neighborhood") {
    return textResult(JSON.stringify(neighborhood(root, args.path, args.depth, args.limit), null, 2));
  }
  throw new Error(`Unknown tool: ${name}`);
}

module.exports = {
  callTool,
  tools,
};
