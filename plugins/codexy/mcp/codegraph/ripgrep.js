"use strict";

const { spawn } = require("child_process");
const { includeLine, isCodeFile, resultLimit } = require("./files");

function rgLines(args, cwd, limit, shouldInclude = includeLine) {
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
        if (line && shouldInclude(line)) lines.push(line);
        if (settleIfLimited()) return;
      }
    });
    child.on("error", (error) => settle(String(error.message || error)));
    child.on("close", (code) => {
      if (settled) return;
      if (pending && shouldInclude(pending)) lines.push(pending);
      if (code === 0 || code === 1) {
        settle(lines.slice(0, limit).join("\n"));
        return;
      }
      settle(`Command failed: rg ${args.join(" ")}`);
    });
  });
}

async function listCodeFiles(root, limit = 400) {
  const output = await rgLines(["--files"], root, resultLimit(limit), isCodeFile);
  return output
    .split(/\r?\n/)
    .filter(Boolean)
    .filter(isCodeFile);
}

module.exports = {
  listCodeFiles,
  rgLines,
};
