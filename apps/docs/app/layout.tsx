import type { Metadata } from "next";
import "@fontsource-variable/geist";
import "@fontsource-variable/geist-mono";
import "./globals.css";
import { Providers } from "@/components/providers";
import { Header, Footer } from "@/components/site-shell";

export const metadata: Metadata = {
  title: { default: "bpmn-viz — BPMN, rendered.", template: "%s · bpmn-viz" },
  description: "Read BPMN 2.0 XML, validate structure and Diagram Interchange, and render deterministic SVG or PNG from pure Rust. Try the real engine live in your browser.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return <html lang="en" suppressHydrationWarning>
    <body><Providers>
      <a className="skip-link" href="#main">Skip to content</a>
      <Header />
      {children}
      <Footer />
    </Providers></body>
  </html>;
}
