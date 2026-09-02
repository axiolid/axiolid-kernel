import assert from "node:assert/strict";
import test from "node:test";

import {
  createGlossaryPlugin,
  parseGlossary,
} from "../.vitepress/glossary-plugin.mjs";

class Token {
  constructor(type, tag = "", nesting = 0) {
    this.type = type;
    this.tag = tag;
    this.nesting = nesting;
    this.attrs = null;
    this.children = null;
    this.content = "";
  }

  attrSet(name, value) {
    this.attrs ??= [];
    const current = this.attrs.find((entry) => entry[0] === name);
    if (current) current[1] = value;
    else this.attrs.push([name, value]);
  }
}

const source = `# Glossary

## B-rep {#b-rep}

**Boundary representation.** A solid represented by connected topology and supporting geometry.

## DAG {#dag}

**Directed acyclic graph.** A directed graph with no directed cycle.
`;

test("parseGlossary reads one canonical term catalog", () => {
  assert.deepEqual(parseGlossary(source), [
    {
      term: "B-rep",
      slug: "b-rep",
      definition: "Boundary representation. A solid represented by connected topology and supporting geometry.",
    },
    {
      term: "DAG",
      slug: "dag",
      definition: "Directed acyclic graph. A directed graph with no directed cycle.",
    },
  ]);
});

test("parseGlossary rejects duplicate terms and missing definitions", () => {
  assert.throws(() => parseGlossary("## DAG {#dag}\n\nOne.\n\n## DAG {#dag-2}\n\nTwo."), /duplicate/i);
  assert.throws(() => parseGlossary("## DAG {#dag}\n"), /definition/i);
});

test("plugin links only first prose use and skips headings, links, and code", () => {
  const entries = parseGlossary(source);
  let transform;
  const md = {
    core: {
      ruler: {
        after(_anchor, _name, callback) {
          transform = callback;
        },
      },
    },
  };
  createGlossaryPlugin(entries)(md);

  const text = (content) => Object.assign(new Token("text"), { content });
  const code = Object.assign(new Token("code_inline", "code"), { content: "DAG" });
  const inline = (children) => Object.assign(new Token("inline"), { children });
  const existingOpen = new Token("link_open", "a", 1);
  const existingClose = new Token("link_close", "a", -1);
  const state = {
    Token,
    env: { relativePath: "architecture.md" },
    tokens: [
      new Token("heading_open", "h2", 1),
      inline([text("DAG architecture")]),
      new Token("heading_close", "h2", -1),
      inline([
        text("A DAG contains another DAG and a B-rep."),
        code,
        existingOpen,
        text("B-rep"),
        existingClose,
      ]),
    ],
  };

  transform(state);
  const heading = state.tokens[1].children;
  assert.deepEqual(heading.map((token) => token.type), ["text"]);

  const prose = state.tokens[3].children;
  assert.equal(prose.filter((token) => token.type === "link_open").length, 3);
  const generated = prose.filter((token) => token.type === "link_open" && token.attrs);
  assert.deepEqual(generated.map((token) => token.attrs.find(([name]) => name === "href")[1]), [
    "/glossary#dag",
    "/glossary#b-rep",
  ]);
  assert.equal(prose.find((token) => token.type === "code_inline").content, "DAG");
  assert.equal(prose.filter((token) => token.type === "text" && token.content.includes("another DAG")).length, 1);
});

test("plugin does not self-link the glossary page", () => {
  const entries = parseGlossary(source);
  let transform;
  createGlossaryPlugin(entries)({ core: { ruler: { after(_a, _n, callback) { transform = callback; } } } });
  const inline = Object.assign(new Token("inline"), { children: [Object.assign(new Token("text"), { content: "DAG" })] });
  const state = { Token, env: { relativePath: "glossary.md" }, tokens: [inline] };
  transform(state);
  assert.deepEqual(inline.children.map((token) => token.type), ["text"]);
});
