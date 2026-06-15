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

module.exports = { fakeLspCommand, withFakeLspCapture };
