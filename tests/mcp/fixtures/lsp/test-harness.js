"use strict";

const { createStdioClient, jsonTextContent } = require("../../stdio-client");

async function withLspClient(assert, lspServer, repoRoot, fn, options = {}) {
  const client = createStdioClient(process.execPath, [lspServer], { cwd: options.cwd || repoRoot, env: options.env });
  try {
    await client.initialize();
    return await fn(client);
  } finally {
    await client.close();
  }
}

function assertStructuredToolResult(assert, response) {
  assert.ifError(response.error);
  return jsonTextContent(response);
}

module.exports = { assertStructuredToolResult, withLspClient };
