<script setup lang="ts">
import { useData } from "vitepress";
import { onMounted, ref, watch } from "vue";

const props = defineProps<{ encoded: string; wide?: boolean }>();
const source = atob(props.encoded);
const target = ref<HTMLElement | null>(null);
const error = ref("");
const { isDark } = useData();
let generation = 0;

async function draw(): Promise<void> {
  if (!target.value) return;
  const current = ++generation;
  error.value = "";
  try {
    const mermaid = (await import("mermaid")).default;
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: "strict",
      theme: "base",
      themeVariables: isDark.value
        ? {
            background: "#1b1b1f",
            primaryColor: isDark.value ? "#172554" : "#eff6ff",
            primaryTextColor: "#e2e8f0",
          }
        : { primaryColor: "#eff6ff" },
    });
    const id = `mermaid-${current}-${Math.random().toString(36).slice(2)}`;
    const rendered = await mermaid.render(id, source);
    if (current !== generation || !target.value) return;
    target.value.innerHTML = rendered.svg;
    rendered.bindFunctions?.(target.value);
  } catch (cause) {
    if (current !== generation) return;
    error.value = cause instanceof Error ? cause.message : String(cause);
  }
}

onMounted(draw);
watch(isDark, draw);
</script>

<template>
  <figure class="diagram-frame" :class="{ 'diagram-frame--wide': props.wide }">
    <div ref="target" class="diagram-canvas mermaid-diagram" />
    <p v-if="props.wide" class="diagram-pan-hint">Swipe diagram horizontally</p>
    <p v-if="error" class="diagram-error">Diagram failed to render: {{ error }}</p>
    <details class="diagram-source">
      <summary>Diagram source</summary>
      <pre><code>{{ source }}</code></pre>
    </details>
  </figure>
</template>
