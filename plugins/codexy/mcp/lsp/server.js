#!/usr/bin/env node
"use strict";

const { createServer } = require("../lib/stdio-mcp");
const { callTool, tools } = require("./tools");

createServer({ name: "codexy-lsp", tools, callTool });
