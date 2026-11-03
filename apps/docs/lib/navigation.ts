export const docs = [
  { slug: "", title: "Introduction", group: "Get started", description: "What bpmn-viz parses, validates and renders — and how the crates fit together." },
  { slug: "quickstart", title: "Quickstart", group: "Get started", description: "Build the CLI and render your first diagram from BPMN DI geometry." },
  { slug: "cli", title: "CLI reference", group: "Guides", description: "Every render, inspect and validate flag, exit code and diagnostic shape." },
  { slug: "layout", title: "Auto-layout", group: "Core concepts", description: "Render models that ship no Diagram Interchange with the native layered engine." },
  { slug: "architecture", title: "Architecture", group: "Core concepts", description: "The pipeline from XML bytes to deterministic SVG, crate by crate." },
  { slug: "bindings", title: "Node.js & WASM", group: "Integrations", description: "Drive the same engine from JavaScript: native addon and portable WASM." },
] as const;
export function docHref(slug: string) { return `/docs/${slug ? `${slug}/` : ""}`; }
