import { readFileSync } from "node:fs";

function plainText(markdown) {
  return markdown
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .replace(/[*_`~]/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

export function parseGlossary(source) {
  const headings = [...source.matchAll(/^##\s+(.+?)\s+\{#([a-z0-9][a-z0-9-]*)\}\s*$/gm)];
  const entries = [];
  const labels = new Set();
  const slugs = new Set();

  for (let index = 0; index < headings.length; index += 1) {
    const match = headings[index];
    const term = match[1].trim();
    const slug = match[2];
    const bodyStart = match.index + match[0].length;
    const bodyEnd = headings[index + 1]?.index ?? source.length;
    const firstParagraph = source
      .slice(bodyStart, bodyEnd)
      .trim()
      .split(/\n\s*\n/, 1)[0] ?? "";
    const definition = plainText(firstParagraph);
    const labelKey = term.toLocaleLowerCase("en-US");

    if (!definition) throw new Error(`Glossary term ${term} lacks a definition`);
    if (labels.has(labelKey)) throw new Error(`Duplicate glossary term: ${term}`);
    if (slugs.has(slug)) throw new Error(`Duplicate glossary slug: ${slug}`);

    labels.add(labelKey);
    slugs.add(slug);
    entries.push({ term, slug, definition });
  }

  if (entries.length === 0) throw new Error("Glossary contains no term definitions");
  return entries;
}

export function loadGlossary(path) {
  return parseGlossary(readFileSync(path, "utf8"));
}

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function glossaryPage(env) {
  const path = String(env?.relativePath ?? env?.path ?? "")
    .replace(/^\//, "")
    .replace(/\.md$/, "");
  return path === "glossary";
}

function linkedTextTokens(state, content, entries, lookup, matcher, seen) {
  const tokens = [];
  let cursor = 0;
  matcher.lastIndex = 0;

  for (let match = matcher.exec(content); match; match = matcher.exec(content)) {
    const value = match[0];
    const entry = lookup.get(value.toLocaleLowerCase("en-US"));
    if (!entry || seen.has(entry.slug)) continue;

    if (match.index > cursor) {
      const before = new state.Token("text", "", 0);
      before.content = content.slice(cursor, match.index);
      tokens.push(before);
    }

    const open = new state.Token("link_open", "a", 1);
    open.attrSet("href", `/glossary#${entry.slug}`);
    open.attrSet("class", "glossary-term");
    open.attrSet("title", entry.definition);
    open.attrSet("data-definition", entry.definition);
    tokens.push(open);

    const label = new state.Token("text", "", 0);
    label.content = value;
    tokens.push(label);
    tokens.push(new state.Token("link_close", "a", -1));

    seen.add(entry.slug);
    cursor = match.index + value.length;
  }

  if (cursor === 0) return null;
  if (cursor < content.length) {
    const after = new state.Token("text", "", 0);
    after.content = content.slice(cursor);
    tokens.push(after);
  }
  return tokens;
}

export function createGlossaryPlugin(entries) {
  const sorted = [...entries].sort((left, right) => right.term.length - left.term.length);
  const lookup = new Map(sorted.map((entry) => [entry.term.toLocaleLowerCase("en-US"), entry]));
  const alternatives = sorted.map((entry) => escapeRegex(entry.term)).join("|");
  const matcher = new RegExp(`(?<![\\p{L}\\p{N}])(?:${alternatives})(?![\\p{L}\\p{N}])`, "giu");

  return (md) => {
    md.core.ruler.after("inline", "axiolid_glossary_links", (state) => {
      if (glossaryPage(state.env)) return;

      const seen = new Set();
      let inHeading = false;
      for (const token of state.tokens) {
        if (token.type === "heading_open") {
          inHeading = true;
          continue;
        }
        if (token.type === "heading_close") {
          inHeading = false;
          continue;
        }
        if (inHeading || token.type !== "inline" || !token.children) continue;

        const transformed = [];
        let linkDepth = 0;
        for (const child of token.children) {
          if (child.type === "link_open") {
            linkDepth += 1;
            transformed.push(child);
            continue;
          }
          if (child.type === "link_close") {
            transformed.push(child);
            linkDepth = Math.max(0, linkDepth - 1);
            continue;
          }
          if (child.type !== "text" || linkDepth > 0) {
            transformed.push(child);
            continue;
          }

          const replacements = linkedTextTokens(state, child.content, sorted, lookup, matcher, seen);
          transformed.push(...(replacements ?? [child]));
        }
        token.children = transformed;
      }
    });
  };
}
