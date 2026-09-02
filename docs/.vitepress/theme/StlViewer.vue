<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";

const props = defineProps<{ encoded: string }>();
const source = atob(props.encoded);
const host = ref<HTMLElement | null>(null);
const error = ref("");
const triangles = ref(0);
const dimensions = ref("");
let dispose: (() => void) | undefined;
let resetCamera: (() => void) | undefined;

function download(): void {
  const url = URL.createObjectURL(new Blob([source], { type: "model/stl" }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = "axiolid-example.stl";
  anchor.click();
  URL.revokeObjectURL(url);
}

onMounted(async () => {
  if (!host.value) return;
  try {
    const THREE = await import("three");
    const [{ STLLoader }, { OrbitControls }] = await Promise.all([
      import("three/examples/jsm/loaders/STLLoader.js"),
      import("three/examples/jsm/controls/OrbitControls.js"),
    ]);
    if (!host.value) return;

    const scene = new THREE.Scene();
    const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.001, 10_000);
    // This viewer renders on demand rather than in an animation loop. Preserve the
    // tiny documentation canvas so screenshots and accessibility QA can inspect
    // the last frame after WebGL swaps its default drawing buffer.
    const renderer = new THREE.WebGLRenderer({
      antialias: true,
      alpha: true,
      preserveDrawingBuffer: true,
    });
    renderer.setClearAlpha(0);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    host.value.appendChild(renderer.domElement);

    const bytes = Uint8Array.from(source, (character) => character.charCodeAt(0));
    const geometry = new STLLoader().parse(bytes.buffer);
    geometry.computeVertexNormals();
    geometry.computeBoundingBox();
    const bounds = geometry.boundingBox;
    if (!bounds || bounds.isEmpty()) throw new Error("STL contains no finite geometry");

    const position = geometry.getAttribute("position");
    const center = new THREE.Vector3();
    for (let index = 0; index < position.count; index += 1) {
      center.x += position.getX(index);
      center.y += position.getY(index);
      center.z += position.getZ(index);
    }
    center.divideScalar(position.count);
    const size = bounds.getSize(new THREE.Vector3());
    geometry.translate(-center.x, -center.y, -center.z);
    triangles.value = geometry.getAttribute("position").count / 3;
    dimensions.value = `${size.x.toPrecision(3)} × ${size.y.toPrecision(3)} × ${size.z.toPrecision(3)}`;

    const material = new THREE.MeshStandardMaterial({
      color: 0x4f7df3,
      metalness: 0.05,
      roughness: 0.68,
      side: THREE.DoubleSide,
    });
    const mesh = new THREE.Mesh(geometry, material);
    scene.add(mesh);
    const edgeGeometry = new THREE.EdgesGeometry(geometry, 1);
    const edgeMaterial = new THREE.LineBasicMaterial({
      color: 0x1e3a8a,
      opacity: 0.65,
      transparent: true,
    });
    scene.add(new THREE.LineSegments(edgeGeometry, edgeMaterial));
    scene.add(new THREE.HemisphereLight(0xffffff, 0x334155, 2.2));
    const key = new THREE.DirectionalLight(0xffffff, 2.5);
    key.position.set(3, 4, 5);
    scene.add(key);

    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = false;
    controls.minZoom = 0.5;
    controls.maxZoom = 8;
    controls.addEventListener("change", render);
    const extent = Math.max(size.x, size.y, size.z, 1e-6);
    resetCamera = () => {
      camera.position.set(extent * 1.65, extent * -1.2, extent * 2.05);
      camera.zoom = 1;
      camera.near = extent / 1000;
      camera.far = extent * 100;
      camera.updateProjectionMatrix();
      controls.target.set(0, 0, 0);
      controls.update();
      render();
    };

    function render(): void {
      if (!host.value) return;
      const width = host.value.clientWidth;
      const height = Math.max(280, Math.min(480, width * 0.62));
      renderer.setSize(width, height, false);
      const half = extent * 0.8;
      const aspect = width / height;
      camera.left = -half * aspect;
      camera.right = half * aspect;
      camera.top = half;
      camera.bottom = -half;
      camera.updateProjectionMatrix();
      camera.lookAt(0, 0, 0);
      renderer.render(scene, camera);
    }

    const resize = new ResizeObserver(render);
    resize.observe(host.value);
    resetCamera();
    dispose = () => {
      resize.disconnect();
      controls.dispose();
      edgeGeometry.dispose();
      edgeMaterial.dispose();
      geometry.dispose();
      material.dispose();
      renderer.dispose();
      renderer.domElement.remove();
    };
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  }
});

onBeforeUnmount(() => dispose?.());
</script>

<template>
  <figure class="diagram-frame stl-frame">
    <div ref="host" class="stl-canvas" role="img" aria-label="Interactive STL geometry model" tabindex="0" />
    <p v-if="error" class="diagram-error">3D model failed to render: {{ error }}</p>
    <figcaption class="stl-toolbar">
      <span v-if="!error" class="stl-stats">{{ triangles }} triangles · bounds {{ dimensions }}</span>
      <span class="stl-actions">
        <button type="button" @click="resetCamera?.()">Reset view</button>
        <button type="button" @click="download">Download STL</button>
      </span>
    </figcaption>
    <p class="stl-hint">Drag to rotate. Scroll or pinch to zoom. Geometry and equations below are the normative explanation.</p>
    <details class="diagram-source">
      <summary>STL source</summary>
      <pre><code>{{ source }}</code></pre>
    </details>
  </figure>
</template>
