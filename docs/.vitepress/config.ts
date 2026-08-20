import { defineConfig } from "vitepress";

const adrs = [
  ["0002-hardware-abstraction-and-backend-selection", 2, "Hardware and backends"],
  ["0003-pure-rust-mesh-boolean", 3, "Pure-Rust mesh booleans"],
  ["0004-package-layout-and-backend-features", 4, "Package layout"],
  ["0009-layered-geometry-dag", 9, "Layered geometry DAG"],
  ["0011-native-accelerator-backends-out-of-tree", 11, "Native accelerators"],
  ["0012-scalar-reference-ownership", 12, "Scalar reference"],
  ["0013-deferred-performance-techniques", 13, "Deferred performance work"],
  ["0014-adopt-boolmesh-mesh-boolean", 14, "Boolmesh provider"],
  ["0015-adopt-earcut-polygon-triangulation", 15, "Earcut triangulation"],
  ["0016-predicate-ownership-and-adopted-implementations", 16, "Predicate ownership"],
] as const;

const adrIcon = `<svg class="adr-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/><path d="M8 13h8M8 17h6"/></svg>`;

function adrSidebarItem([file, number, title]: (typeof adrs)[number]) {
  return {
    text: `<span class="adr-sidebar-label">${adrIcon}<span class="adr-sidebar-title">${title}</span><span class="adr-sidebar-number">${number}</span></span>`,
    link: `/adr/${file}`,
  };
}

export default defineConfig({
  title: "Axiolid",
  description: "A pure-Rust, format-agnostic geometry kernel.",
  base: "/axiolid-kernel/",
  srcExclude: ["adr/_template.md"],
  markdown: { html: false },
  cleanUrls: true,
  lastUpdated: true,
  head: [
    ["meta", { name: "theme-color", content: "#111827" }],
    ["meta", { property: "og:title", content: "Axiolid" }],
    ["meta", { property: "og:description", content: "A pure-Rust, format-agnostic geometry kernel." }],
  ],
  themeConfig: {
    logo: "/mark.svg",
    nav: [
      { text: "Guide", link: "/guide/getting-started" },
      { text: "Architecture", link: "/architecture" },
      { text: "Capabilities", link: "/capabilities" },
      { text: "API", link: "https://docs.rs/axiolid" },
      { text: "GitHub", link: "https://github.com/axiolid/axiolid-kernel" },
    ],
    sidebar: [
      {
        text: "Start here",
        items: [
          { text: "Overview", link: "/" },
          { text: "Getting started", link: "/guide/getting-started" },
          { text: "Capabilities", link: "/capabilities" },
          { text: "Architecture", link: "/architecture" },
          { text: "Crate map", link: "/reference/crates" },
        ],
      },
      {
        text: "Project",
        items: [
          { text: "Changelog", link: "/CHANGELOG" },
          { text: "Research", link: "/research/geometry-kernel-capability-comparison" },
          { text: "Licensing", link: "/guide/licensing" },
          { text: "Contributing", link: "/guide/contributing" },
        ],
      },
      {
        text: `<span class="adr-section-label">${adrIcon}<span>Architecture decisions</span></span>`,
        items: adrs.map(adrSidebarItem),
      },
    ],
    socialLinks: [{ icon: "github", link: "https://github.com/axiolid/axiolid-kernel" }],
    footer: {
      message: "Released under the Mozilla Public License 2.0.",
      copyright: "Copyright © 2026 Axiolid contributors",
    },
    search: { provider: "local" },
  },
});
