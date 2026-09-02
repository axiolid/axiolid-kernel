import DefaultTheme from "vitepress/theme";

import MermaidDiagram from "./MermaidDiagram.vue";
import StlViewer from "./StlViewer.vue";
import "./custom.css";

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component("MermaidDiagram", MermaidDiagram);
    app.component("StlViewer", StlViewer);
  },
};
