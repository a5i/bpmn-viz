import { cp, mkdir, rm, stat } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

// apps/docs lives three levels below the repository root; the committed
// wasm-pack `web` artifact (ADR-0009, TASK-020) is the playground engine.
const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const source = join(repoRoot, "crates", "bpmn-wasm", "pkg-web");
const destination = new URL("../public/runtime/", import.meta.url);

try {
  await stat(join(source, "bpmn_wasm_bg.wasm"));
} catch {
  console.error("Missing WASM artifact crates/bpmn-wasm/pkg-web.\nRun `npm --prefix crates/bpmn-wasm run build:web` (requires wasm-pack), then retry.");
  process.exit(1);
}

await rm(destination, { recursive: true, force: true });
await mkdir(destination, { recursive: true });
for (const file of ["bpmn_wasm.js", "bpmn_wasm_bg.wasm"]) {
  await cp(join(source, file), new URL(file, destination));
}
console.log("Prepared bpmn-wasm browser runtime (no native addon).");
