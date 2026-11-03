import type { Metadata } from "next";
import { FlaskConical, ArrowUpRight } from "lucide-react";
import Link from "next/link";
import { Playground } from "@/components/playground";

export const metadata: Metadata = { title: "Playground" };
export default function PlaygroundPage() {
  return <main id="main" className="playground-page page-container"><div className="eyebrow"><FlaskConical size={14} /> YOUR BROWSER IS THE LAB</div><div className="playground-page-heading"><div><h1>Less guessing. More rendering.</h1><p>Four hands-on examples. Real BPMN models, editable XML, and the bpmn-viz engine compiled to WebAssembly.</p></div><Link className="text-link" href="/docs/layout/">Auto-layout guide <ArrowUpRight size={15} /></Link></div><Playground /><div className="playground-help"><div><span>01</span><strong>Edit the model</strong><p>The diagram renders automatically — point a flow at a missing id and watch the diagnostics appear.</p></div><div><span>02</span><strong>Choose how to render</strong><p>Follow the provided DI, or let the layered engine generate geometry.</p></div><div><span>03</span><strong>Read the diagnostics</strong><p>Stable codes, severities and source locations — the same report the CLI emits.</p></div></div><p className="playground-limits">Examples render automatically and re-render as you edit; the render button forces an immediate re-render. Reset restores the current example; edits are temporary and reset when you leave this page. The WASM build parses, validates, lays out and renders SVG — PNG rasterization is native-only (ADR-0009); browsers rasterize the SVG on canvas. Layout runs under the engine&apos;s documented node and flow limits.</p></main>;
}
