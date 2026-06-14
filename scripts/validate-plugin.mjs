#!/usr/bin/env node
import { constants } from "node:fs";
import { access, readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const semverPattern = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;

const expected = {
  name: "codexy",
  version: "0.1.0",
  author: "Eunsoo Lee",
  license: "MIT",
  repository: "https://github.com/eunsoogi/codexy"
};

function fail(message) {
  throw new Error(message);
}

async function readJson(relativePath) {
  const absolutePath = resolve(root, relativePath);
  const raw = await readFile(absolutePath, "utf8");
  if (raw.includes("[TODO:")) {
    fail(`${relativePath} contains a TODO placeholder`);
  }
  try {
    return JSON.parse(raw);
  } catch (error) {
    fail(`${relativePath} is not readable JSON: ${error.message}`);
  }
}

async function pathExists(relativePath) {
  try {
    await access(resolve(root, relativePath), constants.F_OK);
    return true;
  } catch {
    return false;
  }
}

function assertEqual(actual, expectedValue, label) {
  if (actual !== expectedValue) {
    fail(`${label} must be ${JSON.stringify(expectedValue)}, got ${JSON.stringify(actual)}`);
  }
}

function assertHttps(value, label) {
  if (typeof value !== "string" || !value.startsWith("https://")) {
    fail(`${label} must be an https URL`);
  }
}

function assertNonEmptyString(value, label) {
  if (typeof value !== "string" || value.length === 0) {
    fail(`${label} must be a non-empty string`);
  }
}

function assertStringArray(value, label) {
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) {
    fail(`${label} must be an array of strings`);
  }
}

const plugin = await readJson(".codex-plugin/plugin.json");
const packageJson = await readJson("package.json");
const marketplace = await readJson("marketplace.json");

assertEqual(plugin.name, expected.name, "plugin name");
assertEqual(plugin.version, expected.version, "plugin version");
assertEqual(plugin.license, expected.license, "plugin license");
assertEqual(plugin.author?.name, expected.author, "plugin author.name");
assertEqual(plugin.repository, expected.repository, "plugin repository");

if (!semverPattern.test(plugin.version)) {
  fail("plugin version must be strict semver");
}

for (const unsupportedField of ["apps", "hooks", "mcpServers", "skills"]) {
  if (unsupportedField in plugin) {
    fail(`plugin.json must not declare ${unsupportedField} until its companion files exist`);
  }
}

for (const [field, value] of Object.entries({
  homepage: plugin.homepage,
  repository: plugin.repository,
  "interface.websiteURL": plugin.interface?.websiteURL,
  "interface.privacyPolicyURL": plugin.interface?.privacyPolicyURL,
  "interface.termsOfServiceURL": plugin.interface?.termsOfServiceURL
})) {
  assertHttps(value, field);
}

for (const [field, value] of Object.entries({
  "interface.displayName": plugin.interface?.displayName,
  "interface.shortDescription": plugin.interface?.shortDescription,
  "interface.longDescription": plugin.interface?.longDescription,
  "interface.developerName": plugin.interface?.developerName,
  "interface.category": plugin.interface?.category,
  "interface.brandColor": plugin.interface?.brandColor
})) {
  assertNonEmptyString(value, field);
}

assertStringArray(plugin.keywords, "plugin keywords");
assertStringArray(plugin.interface?.capabilities, "plugin interface.capabilities");
assertStringArray(plugin.interface?.defaultPrompt, "plugin interface.defaultPrompt");

if (plugin.interface.defaultPrompt.length > 3) {
  fail("plugin interface.defaultPrompt must contain at most 3 prompts");
}

for (const prompt of plugin.interface.defaultPrompt) {
  if (prompt.length > 128) {
    fail("plugin interface.defaultPrompt entries must be at most 128 characters");
  }
}

for (const reference of [plugin.interface.composerIcon, plugin.interface.logo]) {
  assertNonEmptyString(reference, "plugin interface asset reference");
  if (!(await pathExists(reference))) {
    fail(`manifest reference does not exist: ${reference}`);
  }
}

assertEqual(packageJson.name, expected.name, "package name");
assertEqual(packageJson.version, expected.version, "package version");
assertEqual(packageJson.license, expected.license, "package license");
assertEqual(packageJson.author, expected.author, "package author");
assertEqual(packageJson.repository?.url, "git+https://github.com/eunsoogi/codexy.git", "package repository.url");

assertEqual(marketplace.name, expected.name, "marketplace name");
assertEqual(marketplace.interface?.displayName, "Codexy", "marketplace interface.displayName");
assertEqual(marketplace.metadata?.version, expected.version, "marketplace metadata.version");

const entry = marketplace.plugins?.find((candidate) => candidate.name === expected.name);
if (!entry) {
  fail("marketplace must include a codexy plugin entry");
}

assertEqual(entry.source?.source, "local", "marketplace codexy source.source");
assertEqual(entry.source?.path, ".", "marketplace codexy source.path");
assertEqual(entry.policy?.installation, "AVAILABLE", "marketplace codexy policy.installation");
assertEqual(entry.policy?.authentication, "ON_INSTALL", "marketplace codexy policy.authentication");
assertEqual(entry.category, "Developer Tools", "marketplace codexy category");

console.log("plugin validation passed");
