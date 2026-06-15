#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");
const { createServer, textResult } = require("../lib/stdio-mcp");

const pluginRoot = path.resolve(__dirname, "..", "..");
const codeExtensions = new Set([".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs", ".py", ".go", ".rs", ".rb", ".java", ".kt"]);

function repoRoot(inputRoot) {
  const candidate = inputRoot ? path.resolve(inputRoot) : process.cwd();
  return fs.existsSync(candidate) ? candidate : process.cwd();
}

function rg(args, cwd) {
  try {
    return execFileSync("rg", args, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
  } catch (error) {
    return error.status === 1 ? "" : String(error.message || error);
  }
}

function listCodeFiles(root, limit = 400) {
  return rg(["--files"], root)
    .split(/\r?\n/)
    .filter(Boolean)
    .filter((file) => codeExtensions.has(path.extname(file)))
    .slice(0, limit);
}

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
];

async function callTool(name, args) {
  const root = repoRoot(args.root || pluginRoot);
  if (name === "codegraph_overview") {
    const files = listCodeFiles(root, args.limit || 400);
    const edges = files.flatMap((file) =>
      importsFor(path.join(root, file)).map((edge) => ({ file, ...edge }))
    );
    return textResult(JSON.stringify({ root, fileCount: files.length, files, importEdges: edges.slice(0, 300) }, null, 2));
  }
  if (name === "codegraph_search") {
    const output = rg(["-n", "--glob", "!node_modules", "--glob", "!.git", "-e", args.query], root)
      .split(/\r?\n/)
      .filter(Boolean)
      .slice(0, args.limit || 80);
    return textResult(output.join("\n"));
  }
  if (name === "codegraph_neighbors") {
    return textResult(JSON.stringify(importsFor(path.join(root, args.path)), null, 2));
  }
  throw new Error(`Unknown tool: ${name}`);
}

createServer({ name: "codexy-codegraph", tools, callTool });
