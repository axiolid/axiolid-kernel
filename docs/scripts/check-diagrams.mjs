import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const root = process.argv[2] ?? ".";
const files = [];
function walk(dir) {
  for (const name of readdirSync(dir)) {
    if (["node_modules", ".vitepress", "dist"].includes(name)) continue;
    const path = join(dir, name);
    const stat = statSync(path);
    if (stat.isDirectory()) walk(path);
    else if (name.endsWith(".md")) files.push(path);
  }
}
walk(root);

const scalarPattern = "[+-]?(?:(?:\\d+(?:\\.\\d*)?)|(?:\\.\\d+))(?:[eE][+-]?\\d+)?";
const vectorPattern = new RegExp(`^(${scalarPattern})\\s+(${scalarPattern})\\s+(${scalarPattern})$`);
const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a, b) => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
];
const subtract = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const vertexKey = (vertex) => vertex.map((value) => Object.is(value, -0) ? "0" : String(value)).join(",");

function parseVector(text, context) {
  const match = vectorPattern.exec(text);
  if (!match) throw new Error(`${context}: expected exactly three numeric coordinates`);
  const vector = match.slice(1).map(Number);
  if (!vector.every(Number.isFinite)) throw new Error(`${context}: coordinates must be finite`);
  return vector;
}

function validateAsciiStl(source, label) {
  const lines = source.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  let cursor = 0;
  const next = (expected) => {
    const line = lines[cursor];
    if (line === undefined) throw new Error(`${label}: expected ${expected}, found end of fence`);
    cursor += 1;
    return line;
  };

  const solidMatch = /^solid\s+(\S(?:.*\S)?)$/.exec(next("named solid"));
  if (!solidMatch) throw new Error(`${label}: STL must start with a named solid`);
  const solidName = solidMatch[1];
  const facets = [];

  while (cursor < lines.length && !lines[cursor].startsWith("endsolid")) {
    const normalLine = next("facet normal");
    const normalMatch = /^facet\s+normal\s+(.+)$/.exec(normalLine);
    if (!normalMatch) throw new Error(`${label}: expected facet normal, found '${normalLine}'`);
    const normal = parseVector(normalMatch[1], `${label}: facet normal`);
    if (dot(normal, normal) === 0) throw new Error(`${label}: facet normal must be nonzero`);

    const outer = next("outer loop");
    if (outer !== "outer loop") throw new Error(`${label}: expected outer loop, found '${outer}'`);
    const vertices = [];
    for (let index = 0; index < 3; index += 1) {
      const vertexLine = next(`vertex ${index + 1}`);
      const vertexMatch = /^vertex\s+(.+)$/.exec(vertexLine);
      if (!vertexMatch) throw new Error(`${label}: expected vertex, found '${vertexLine}'`);
      vertices.push(parseVector(vertexMatch[1], `${label}: vertex`));
    }
    const endloop = next("endloop");
    if (endloop !== "endloop") throw new Error(`${label}: expected endloop, found '${endloop}'`);
    const endfacet = next("endfacet");
    if (endfacet !== "endfacet") throw new Error(`${label}: expected endfacet, found '${endfacet}'`);
    facets.push({ normal, vertices });
  }

  const endMatch = /^endsolid(?:\s+(\S(?:.*\S)?))?$/.exec(next("endsolid"));
  if (!endMatch) throw new Error(`${label}: malformed endsolid`);
  if (endMatch[1] !== undefined && endMatch[1] !== solidName) {
    throw new Error(`${label}: solid and endsolid names differ`);
  }
  if (cursor !== lines.length) throw new Error(`${label}: content follows endsolid`);
  if (facets.length < 4) throw new Error(`${label}: STL must contain at least four facets`);

  const edges = new Map();
  let signedVolumeTimesSix = 0;
  for (const [facetIndex, facet] of facets.entries()) {
    const [a, b, c] = facet.vertices;
    const geometricNormal = cross(subtract(b, a), subtract(c, a));
    if (dot(geometricNormal, geometricNormal) === 0) {
      throw new Error(`${label}: facet ${facetIndex + 1} is degenerate`);
    }
    if (dot(geometricNormal, facet.normal) <= 0) {
      throw new Error(`${label}: facet ${facetIndex + 1} normal disagrees with vertex winding`);
    }
    signedVolumeTimesSix += dot(a, cross(b, c));

    for (const [from, to] of [[a, b], [b, c], [c, a]]) {
      const fromKey = vertexKey(from);
      const toKey = vertexKey(to);
      if (fromKey === toKey) throw new Error(`${label}: facet ${facetIndex + 1} has a zero-length edge`);
      const forward = fromKey < toKey;
      const key = forward ? `${fromKey}|${toKey}` : `${toKey}|${fromKey}`;
      const edge = edges.get(key) ?? { count: 0, orientation: 0 };
      edge.count += 1;
      edge.orientation += forward ? 1 : -1;
      edges.set(key, edge);
    }
  }

  for (const [key, edge] of edges) {
    if (edge.count !== 2 || edge.orientation !== 0) {
      throw new Error(`${label}: mesh is not a closed consistently oriented 2-manifold at edge ${key}`);
    }
  }
  if (!(signedVolumeTimesSix > 0) || !Number.isFinite(signedVolumeTimesSix)) {
    throw new Error(`${label}: closed mesh must have finite positive signed volume`);
  }
}

const errors = [];
let mermaid = 0;
let stl = 0;
for (const path of files) {
  const content = readFileSync(path, "utf8");
  const label = relative(root, path);
  for (const match of content.matchAll(/```mermaid[^\n]*\n([\s\S]*?)```/g)) {
    mermaid += 1;
    if (!/^\s*accTitle:\s*\S+/m.test(match[1])) errors.push(`${label}: Mermaid fence lacks accTitle`);
    if (!/^\s*accDescr:\s*\S+/m.test(match[1])) errors.push(`${label}: Mermaid fence lacks accDescr`);
  }
  for (const match of content.matchAll(/```stl[^\n]*\n([\s\S]*?)```/g)) {
    stl += 1;
    try {
      validateAsciiStl(match[1], label);
    } catch (error) {
      errors.push(error instanceof Error ? error.message : `${label}: invalid ASCII STL`);
    }
  }
  for (let index = 0; index < content.length; index += 1) {
    const code = content.charCodeAt(index);
    if (code < 32 && code !== 9 && code !== 10) errors.push(`${label}: forbidden control byte 0x${code.toString(16)}`);
  }
}
if (mermaid === 0) errors.push("no Mermaid fences found");
if (errors.length) {
  console.error(errors.join("\n"));
  process.exit(1);
}
console.log(`DIAGRAM_SOURCE_CHECK=PASS markdown=${files.length} mermaid=${mermaid} stl=${stl}`);
