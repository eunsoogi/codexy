#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");
const { execFileSync, spawn } = require("child_process");
const { createServer, textResult } = require("../lib/stdio-mcp");

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

function resultLimit(inputLimit) {
  const parsed = Number(inputLimit);
  return Number.isFinite(parsed) && parsed > 0 ? Math.floor(parsed) : 80;
}

function rgLines(args, cwd, limit) {
  return new Promise((resolve) => {
    const child = spawn("rg", args, { cwd, stdio: ["ignore", "pipe", "ignore"] });
    const lines = [];
    let pending = "";
    let settled = false;

    function settle(output) {
      if (settled) return;
      settled = true;
      resolve(output);
    }

    function settleIfLimited() {
      if (lines.length < limit) return false;
      child.kill();
      settle(lines.slice(0, limit).join("\n"));
      return true;
    }

    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      if (settled) return;
      pending += chunk;
      const parts = pending.split(/\r?\n/);
      pending = parts.pop() || "";
      for (const line of parts) {
        if (line) lines.push(line);
        if (settleIfLimited()) return;
      }
    });
    child.on("error", (error) => settle(String(error.message || error)));
    child.on("close", (code) => {
      if (settled) return;
      if (pending) lines.push(pending);
      if (code === 0 || code === 1) {
        settle(lines.slice(0, limit).join("\n"));
        return;
      }
      settle(`Command failed: rg ${args.join(" ")}`);
    });
  });
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
  const root = repoRoot(args.root);
  if (name === "codegraph_overview") {
    const files = listCodeFiles(root, args.limit || 400);
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
  throw new Error(`Unknown tool: ${name}`);
}

createServer({ name: "codexy-codegraph", tools, callTool });
