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
    if (!/^solid\s+\S+/m.test(match[1]) || !/^endsolid(?:\s+\S+)?\s*$/m.test(match[1])) {
      errors.push(`${label}: STL fence must be a complete ASCII solid`);
    }
    if ((match[1].match(/^\s*facet normal\b/gm) ?? []).length < 4) {
      errors.push(`${label}: STL fence must contain at least four facets`);
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
