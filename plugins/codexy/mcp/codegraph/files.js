"use strict";

const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

const codeExtensions = new Set([".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs", ".py", ".go", ".rs", ".rb", ".java", ".kt"]);

function repoRoot(inputRoot) {
  const candidate = inputRoot ? path.resolve(inputRoot) : process.cwd();
  return fs.existsSync(candidate) ? candidate : process.cwd();
}

function resultLimit(inputLimit) {
  const parsed = Number(inputLimit);
  return Number.isFinite(parsed) && parsed > 0 ? Math.floor(parsed) : 80;
}

function includeLine(line) {
  return Boolean(line);
}

function isCodeFile(file) {
  return codeExtensions.has(path.extname(file));
}

function toPosix(file) {
  return file.split(path.sep).join("/");
}

function ignoredByGit(root, relativePaths) {
  if (relativePaths.length === 0) return new Set();
  const input = `${relativePaths.join("\0")}\0`;
  const result = spawnSync("git", ["-C", root, "check-ignore", "-z", "--stdin"], {
    input,
    encoding: "utf8",
    maxBuffer: 1024 * 1024,
  });
  if (result.status === 1) return new Set();
  if (result.status !== 0) return new Set();
  return new Set(result.stdout.split("\0").filter(Boolean));
}

function walkCodeFiles(root) {
  const files = [];
  const ignored = new Set([".git", "node_modules"]);

  function walk(dir) {
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    const relativeEntries = entries.map((entry) => toPosix(path.relative(root, path.join(dir, entry.name))));
    const gitIgnored = ignoredByGit(root, relativeEntries);

    for (const [index, entry] of entries.entries()) {
      const relative = relativeEntries[index];
      if (ignored.has(entry.name) || gitIgnored.has(relative)) continue;
      const absolute = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        walk(absolute);
        continue;
      }
      if (entry.isFile() && isCodeFile(absolute)) {
        files.push(relative);
      }
    }
  }

  walk(root);
  return files.sort();
}

function unique(values) {
  return Array.from(new Set(values)).sort();
}

module.exports = {
  codeExtensions,
  includeLine,
  isCodeFile,
  ignoredByGit,
  repoRoot,
  resultLimit,
  toPosix,
  unique,
  walkCodeFiles,
};
