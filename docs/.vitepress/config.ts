import { defineConfig } from "vitepress";

const adrs = [
  ["0002-hardware-abstraction-and-backend-selection", "0002 Hardware and backends"],
  ["0003-pure-rust-mesh-boolean", "0003 Pure-Rust mesh booleans"],
  ["0004-package-layout-and-backend-features", "0004 Package layout"],
  ["0009-layered-geometry-dag", "0009 Layered geometry DAG"],
  ["0011-native-accelerator-backends-out-of-tree", "0011 Native accelerators"],
  ["0012-scalar-reference-ownership", "0012 Scalar reference"],
  ["0013-deferred-performance-techniques", "0013 Deferred performance work"],
  ["0014-adopt-boolmesh-mesh-boolean", "0014 Boolmesh provider"],
  ["0015-adopt-earcut-polygon-triangulation", "0015 Earcut triangulation"],
  ["0016-predicate-ownership-and-adopted-implementations", "0016 Predicate ownership"],
] as const;

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
      { text: "Architecture decisions", items: adrs.map(([file, text]) => ({ text, link: `/adr/${file}` })) },
    ],
    socialLinks: [{ icon: "github", link: "https://github.com/axiolid/axiolid-kernel" }],
    footer: {
      message: "Released under the Mozilla Public License 2.0.",
      copyright: "Copyright © 2026 Axiolid contributors",
    },
    search: { provider: "local" },
  },
});
