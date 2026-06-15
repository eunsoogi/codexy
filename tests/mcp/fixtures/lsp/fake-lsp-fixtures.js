"use strict";

const fs = require("fs");
const os = require("os");
const path = require("path");

const fakeLspServer = path.join(__dirname, "fake-lsp-server.js");

function fakeLspCommand() {
  return [process.execPath, fakeLspServer];
}

async function withFakeLspCapture(prefix, fn) {
  const capturePath = path.join(os.tmpdir(), `${prefix}-${process.pid}-${Date.now()}.json`);
  try {
    return await fn(capturePath);
  } finally {
    fs.rmSync(capturePath, { force: true });
  }
}

function readCapture(capturePath) {
  return JSON.parse(fs.readFileSync(capturePath, "utf8"));
}

async function withMarkerlessWorkspace(prefix, relativePath, fn) {
  const workspaceRoot = fs.mkdtempSync(path.join(os.tmpdir(), `${prefix}-${process.pid}-`));
  const filePath = path.join(workspaceRoot, relativePath);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, "export const markerless = true;\n");
  try {
    return await fn({ workspaceRoot, filePath });
  } finally {
    fs.rmSync(workspaceRoot, { recursive: true, force: true });
  }
}

module.exports = { fakeLspCommand, readCapture, withFakeLspCapture, withMarkerlessWorkspace };
