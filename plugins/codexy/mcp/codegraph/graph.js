"use strict";
const fs = require("fs");
const path = require("path");
const { codeExtensions, resultLimit, toPosix, unique, walkCodeFiles } = require("./files");
const jsFamilyExtensions = new Set([".js", ".jsx", ".mjs", ".cjs"]);
const jsSourceExtensions = new Set([".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx"]);
const tsSourceExtensions = [".ts", ".tsx"];
function startsRegexLiteral(source, index) {
  for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
    const char = source[cursor];
    if (/\s/.test(char)) continue;
    if ("([{=,:;!&|?+-*%~^<>".includes(char)) return true;
    return /\b(?:return|throw|case|delete|void|typeof|instanceof|in|of|yield|await)$/.test(source.slice(0, cursor + 1));
  }
  return true;
}
function readRegexLiteral(source, index) {
  let output = "/";
  let escaped = false, inClass = false;
  for (index += 1; index < source.length; index += 1) {
    const char = source[index];
    output += char;
    if (escaped) escaped = false;
    else if (char === "\\") escaped = true;
    else if (char === "[") inClass = true;
    else if (char === "]" && inClass) inClass = false;
    else if (char === "/" && !inClass) {
      while (/[A-Za-z]/.test(source[index + 1] || "")) output += source[++index];
      break;
    }
  }
  return { output, index };
}
function codePositionMask(source) {
  const mask = Array(source.length).fill(true), stack = [];
  let mode = "code", quote = null, escaped = false, templateDepth = 0;
  const push = (next) => { stack.push({ mode, templateDepth }); mode = next; escaped = false; };
  const pop = () => { const previous = stack.pop() || { mode: "code", templateDepth: 0 }; mode = previous.mode; templateDepth = previous.templateDepth; quote = null; escaped = false; };
  for (let index = 0; index < source.length; index += 1) {
    const char = source[index];
    if (mode === "string") {
      mask[index] = false; if (escaped) escaped = false;
      else if (char === "\\") escaped = true;
      else if (char === quote) pop();
      continue;
    }
    if (mode === "template") {
      mask[index] = false; if (escaped) escaped = false;
      else if (char === "\\") escaped = true;
      else if (char === "`") pop();
      else if (char === "$" && source[index + 1] === "{") { mask[++index] = false; templateDepth = 1; push("templateExpr"); }
      continue;
    }
    if (mode === "templateExpr" && char === "{") templateDepth += 1;
    else if (mode === "templateExpr" && char === "}" && --templateDepth === 0) { mask[index] = false; pop(); continue; }
    if (char === "\"" || char === "'") { quote = char; push("string"); mask[index] = false; continue; }
    if (char === "`") { push("template"); mask[index] = false; continue; }
    if (char === "/" && source[index + 1] === "/") {
      while (index < source.length && source[index] !== "\n") mask[index++] = false;
      index -= 1; continue;
    }
    if (char === "/" && source[index + 1] === "*") {
      for (; index < source.length && !(source[index] === "*" && source[index + 1] === "/"); index += 1) mask[index] = false;
      mask[index] = false; mask[index + 1] = false; index += 1; continue;
    }
    if (char === "/" && startsRegexLiteral(source, index)) {
      const regex = readRegexLiteral(source, index);
      for (let cursor = index; cursor <= regex.index; cursor += 1) mask[cursor] = false;
      index = regex.index;
    }
  }
  return mask;
}
function parseJavaScriptFile(root, file) {
  const absolute = path.join(root, file);
  const source = fs.readFileSync(absolute, "utf8");
  const mask = codePositionMask(source);
  const imports = [];
  const exports = [];
  const importPatterns = [
    /\bimport\s*(?:[^"'()]*?\s*from\s*)?["']([^"']+)["']/g, /\bimport\s*\(\s*["']([^"']+)["'](?:\s*,[^)]*)?\s*\)/g,
    /\brequire\(\s*["']([^"']+)["']\s*\)/g, /\bexport\s*(?:type\s+)?\*\s*from\s*["']([^"']+)["']/g,
    /\bexport\s*(?:type\s+)?\{[^}]+\}\s*from\s*["']([^"']+)["']/g,
  ];
  const exportPatterns = [/\bexport\s+(?:async\s+)?(?:function|class|const|let|var)\s+([A-Za-z_$][\w$]*)/g, /\bexport\s*(?:type\s+)?\{([^}]+)\}/g];
  for (const pattern of importPatterns) {
    for (const match of source.matchAll(pattern)) {
      if (!mask[match.index]) continue;
      imports.push(match[1]);
    }
  }
  for (const match of source.matchAll(exportPatterns[0])) {
    if (!mask[match.index]) continue;
    exports.push(match[1]);
  }
  for (const match of source.matchAll(exportPatterns[1])) {
    if (!mask[match.index]) continue;
    for (const name of match[1].split(",")) {
      const exported = name.trim().split(/\s+as\s+/).pop();
      if (exported) exports.push(exported);
    }
  }
  return { imports: unique(imports), exports: unique(exports) };
}
const languageRules = {
  ".py": { imports: [/\bfrom\s+(\.+)\s+import\s+([A-Za-z_]\w*)/g, /\bfrom\s+((?:\.+)?[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)\s+import\b/g, /^\s*import\s+([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)/gm], exports: [/\b(?:def|class)\s+([A-Za-z_]\w*)/g] },
  ".go": { imports: [/\bimport\s+(?:\(\s*)?(?:[A-Za-z_]\w*\s+)?["']([^"']+)["']/g, /^\s*(?:[A-Za-z_]\w*\s+)?["'](\.[^"']+)["']/gm], exports: [/\b(?:func|type|var|const)\s+([A-Z]\w*)/g] },
  ".rs": { imports: [/\bmod\s+([A-Za-z_]\w*)\s*;/g, /\buse\s+((?:crate|self|super)::[A-Za-z_]\w*(?:::[A-Za-z_]\w*)*)/g], exports: [/\bpub\s+(?:fn|struct|enum|trait|mod|const|static)\s+([A-Za-z_]\w*)/g] },
  ".rb": { imports: [/\brequire_relative\s+["']([^"']+)["']/g, /\brequire\s+["'](\.[^"']+)["']/g], exports: [/\b(?:class|module|def)\s+([A-Z]\w*|[a-z_]\w*[!?=]?)/g] },
  ".java": { imports: [/\bimport\s+(?:static\s+)?([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)+)\s*;/g], exports: [/\b(?:class|interface|enum|record)\s+([A-Za-z_]\w*)/g] },
  ".kt": { imports: [/\bimport\s+([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)+)/g], exports: [/\b(?:class|interface|object|fun|val|var)\s+([A-Za-z_]\w*)/g] },
};
function normalizeLanguageImport(extension, specifier, file) {
  if (extension === ".py") {
    if (specifier.startsWith(".")) {
      const dots = specifier.match(/^\.+/)[0].length;
      return `./${"../".repeat(Math.max(0, dots - 1))}${specifier.replace(/^\.+/, "").replace(/\./g, "/")}`;
    }
    const relative = path.posix.relative(path.posix.dirname(file), specifier.replace(/\./g, "/"));
    return relative.startsWith(".") ? relative : `./${relative}`;
  }
  if (extension === ".rs") {
    if (specifier.startsWith("crate::")) return `./${path.posix.relative(path.posix.dirname(file), path.posix.join(file.split("/").includes("src") ? "src" : ".", specifier.slice(7).replace(/::/g, "/")))}`;
    if (specifier.startsWith("super::")) return `./../${specifier.slice(7).replace(/::/g, "/")}`;
    if (specifier.startsWith("self::")) return `./${specifier.slice(6).replace(/::/g, "/")}`;
    if ((path.posix.dirname(file) === "src" || path.posix.dirname(file).startsWith("src/")) && !["lib.rs", "main.rs", "mod.rs"].includes(path.posix.basename(file))) return `./${path.posix.basename(file, ".rs")}/${specifier}`;
    return `./${specifier.replace(/::/g, "/")}`;
  }
  if (extension === ".go" && !specifier.startsWith(".")) return specifier;
  if (extension === ".java" || extension === ".kt") return `./${specifier.replace(/\./g, "/")}`;
  return specifier.startsWith(".") ? specifier : `./${specifier}`;
}

function languageMask(source, extension) {
  const mask = codePositionMask(source);
  if (extension === ".py" || extension === ".rb") for (let index = 0; index < source.length; index += 1) {
    if (mask[index] && source[index] === "#") while (index < source.length && source[index] !== "\n") mask[index++] = false;
  }
  return mask;
}

function parseLanguageFile(root, file) {
  const extension = path.extname(file);
  const rules = languageRules[extension];
  if (!rules) return { imports: [], exports: [] };
  const source = fs.readFileSync(path.join(root, file), "utf8");
  const packageName = [".java", ".kt"].includes(extension) && source.match(/^\s*package\s+([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)/m)?.[1];
  const mask = languageMask(source, extension);
  const imports = rules.imports.flatMap((pattern) => Array.from(source.matchAll(pattern)).filter((match) => mask[match.index]).map((match) => {
      const specifier = `${match[1]}${match[2] || ""}`;
      if (packageName && specifier.startsWith(`${packageName}.`)) return `./${specifier.slice(packageName.length + 1).replace(/\./g, "/")}`;
      return normalizeLanguageImport(extension, specifier, file);
    })
  );
  const exports = rules.exports.flatMap((pattern) =>
    Array.from(source.matchAll(pattern)).filter((match) => mask[match.index]).map((match) => match[1])
  );
  return { imports: unique(imports), exports: unique(exports) };
}

function resolveImport(root, fromFile, specifier, indexedFiles) {
  if (!specifier.startsWith(".")) {
    return { to: specifier, resolved: false };
  }

  const fromDir = path.dirname(path.join(root, fromFile));
  const candidate = path.resolve(fromDir, specifier);
  const extension = path.extname(candidate);
  const candidates = extension
    ? [candidate, ...(jsFamilyExtensions.has(extension) ? tsSourceExtensions.map((sourceExtension) => `${candidate.slice(0, -extension.length)}${sourceExtension}`) : [])]
    : [candidate, ...Array.from(codeExtensions, (extension) => `${candidate}${extension}`), ...Array.from(codeExtensions, (extension) => path.join(candidate, `index${extension}`)), path.join(candidate, "__init__.py"), path.join(candidate, "mod.rs")];

  for (const absolute of candidates) {
    if (fs.existsSync(absolute) && fs.statSync(absolute).isFile()) {
      const relative = toPosix(path.relative(root, absolute));
      if (indexedFiles.has(relative)) {
        return { to: relative, resolved: true };
      }
    }
  }

  return { to: specifier, resolved: false };
}

function buildGraph(root, limit) {
  const boundedLimit = resultLimit(limit);
  const allFiles = walkCodeFiles(root);
  const selectedFiles = allFiles.slice(0, boundedLimit);
  const truncated = allFiles.length > selectedFiles.length;
  const indexedFiles = new Set(allFiles);
  const files = selectedFiles.map((file) => {
    const parsed = jsSourceExtensions.has(path.extname(file)) ? parseJavaScriptFile(root, file) : parseLanguageFile(root, file);
    return { path: file, imports: parsed.imports, exports: parsed.exports };
  });
  const edges = files.flatMap((file) =>
    file.imports.map((specifier) => {
      const resolved = resolveImport(root, file.path, specifier, indexedFiles);
      return { from: file.path, to: resolved.to, specifier, resolved: resolved.resolved };
    })
  );

  return { root, files, edges, totalFiles: allFiles.length, limit: boundedLimit, truncated, metadata: { truncated } };
}

function reverseDeps(root, targetPath, limit) {
  const graph = buildGraph(root, Number.MAX_SAFE_INTEGER);
  const normalizedTarget = toPosix(targetPath);
  const dependents = graph.edges
    .filter((edge) => edge.resolved && edge.to === normalizedTarget)
    .map((edge) => ({ path: edge.from, specifier: edge.specifier }))
    .sort((left, right) => left.path.localeCompare(right.path))
    .slice(0, resultLimit(limit));
  return { root, path: normalizedTarget, dependents, limit: resultLimit(limit) };
}

function neighborhood(root, startPath, depth, limit) {
  const graph = buildGraph(root, Number.MAX_SAFE_INTEGER);
  const boundedDepth = Math.max(0, Math.floor(Number(depth) || 1));
  const boundedLimit = resultLimit(limit);
  const start = toPosix(startPath);
  const seen = new Set();
  const queue = [{ path: start, depth: 0 }];
  const nodes = [];
  const edges = [];

  while (queue.length && nodes.length < boundedLimit) {
    const current = queue.shift();
    if (seen.has(current.path)) continue;
    seen.add(current.path);
    nodes.push({ path: current.path });
    if (current.depth >= boundedDepth) continue;

    for (const edge of graph.edges.filter((candidate) => candidate.from === current.path && candidate.resolved)) {
      edges.push(edge);
      if (!seen.has(edge.to)) queue.push({ path: edge.to, depth: current.depth + 1 });
    }
  }
  const returnedNodePaths = new Set(nodes.map((node) => node.path));
  const neighborhoodEdges = edges.filter(
    (edge) => returnedNodePaths.has(edge.from) && returnedNodePaths.has(edge.to)
  );
  const boundedEdges = neighborhoodEdges.slice(0, boundedLimit);
  const hasPendingNeighbors = queue.some((candidate) => !seen.has(candidate.path));

  return { root, path: start, depth: boundedDepth, nodes, edges: boundedEdges, limit: boundedLimit, truncated: hasPendingNeighbors || neighborhoodEdges.length > boundedEdges.length };
}

module.exports = { buildGraph, neighborhood, reverseDeps };
