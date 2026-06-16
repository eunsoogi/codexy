"use strict";

const fs = require("fs");
const path = require("path");
const { codeExtensions, resultLimit, toPosix, unique, walkCodeFiles } = require("./files");
const jsFamilyExtensions = new Set([".js", ".jsx", ".mjs", ".cjs"]);
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

function stripJavaScriptComments(source) {
  let output = "";
  let quote = null;
  let escaped = false;

  for (let index = 0; index < source.length; index += 1) {
    const char = source[index];
    const next = source[index + 1];

    if (quote) {
      output += char;
      if (escaped) escaped = false;
      else if (char === "\\") escaped = true;
      else if (char === quote) quote = null;
      continue;
    }

    if (char === "\"" || char === "'" || char === "`") {
      quote = char;
      output += char;
      continue;
    }

    if (char === "/" && next === "/") {
      while (index < source.length && source[index] !== "\n") index += 1;
      output += "\n"; continue;
    }

    if (char === "/" && next === "*") {
      index += 2;
      while (index < source.length && !(source[index] === "*" && source[index + 1] === "/")) {
        output += source[index] === "\n" ? "\n" : " ";
        index += 1;
      }
      index += 1; continue;
    }

    if (char === "/" && startsRegexLiteral(source, index)) {
      const regex = readRegexLiteral(source, index);
      output += regex.output; index = regex.index; continue;
    }

    output += char;
  }

  return output;
}

function parseJavaScriptFile(root, file) {
  const absolute = path.join(root, file);
  const source = fs.readFileSync(absolute, "utf8");
  const parsedSource = stripJavaScriptComments(source);
  const imports = [];
  const exports = [];
  const importPatterns = [
    /\bimport\s+(?:[^"'()]*?\s+from\s+)?["']([^"']+)["']/g,
    /\brequire\(\s*["']([^"']+)["']\s*\)/g,
    /\bexport\s+(?:type\s+)?\*\s*from\s*["']([^"']+)["']/g,
    /\bexport\s+(?:type\s+)?\{[^}]+\}\s*from\s*["']([^"']+)["']/g,
  ];
  const exportPatterns = [
    /\bexport\s+(?:async\s+)?(?:function|class|const|let|var)\s+([A-Za-z_$][\w$]*)/g,
    /\bexport\s+(?:type\s+)?\{([^}]+)\}/g,
  ];

  for (const pattern of importPatterns) {
    for (const match of parsedSource.matchAll(pattern)) {
      imports.push(match[1]);
    }
  }
  for (const match of parsedSource.matchAll(exportPatterns[0])) {
    exports.push(match[1]);
  }
  for (const match of parsedSource.matchAll(exportPatterns[1])) {
    for (const name of match[1].split(",")) {
      const exported = name.trim().split(/\s+as\s+/).pop();
      if (exported) exports.push(exported);
    }
  }

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
    ? [
        candidate,
        ...(jsFamilyExtensions.has(extension)
          ? tsSourceExtensions.map((sourceExtension) => `${candidate.slice(0, -extension.length)}${sourceExtension}`)
          : []),
      ]
    : [
        candidate,
        ...Array.from(codeExtensions, (extension) => `${candidate}${extension}`),
        ...Array.from(codeExtensions, (extension) => path.join(candidate, `index${extension}`)),
      ];

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
    const parsed = [".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx"].includes(path.extname(file))
      ? parseJavaScriptFile(root, file)
      : { imports: [], exports: [] };
    return { path: file, imports: parsed.imports, exports: parsed.exports };
  });
  const selected = new Set(selectedFiles);
  const edges = files.flatMap((file) =>
    file.imports.map((specifier) => {
      const resolved = resolveImport(root, file.path, specifier, indexedFiles);
      return {
        from: file.path,
        to: resolved.to,
        specifier,
        resolved: resolved.resolved,
      };
    })
  );

  return {
    root,
    files,
    edges,
    totalFiles: allFiles.length,
    limit: boundedLimit,
    truncated,
    metadata: { truncated },
  };
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

  return {
    root,
    path: start,
    depth: boundedDepth,
    nodes,
    edges: boundedEdges,
    limit: boundedLimit,
    truncated: hasPendingNeighbors || neighborhoodEdges.length > boundedEdges.length,
  };
}

module.exports = {
  buildGraph,
  neighborhood,
  reverseDeps,
};
