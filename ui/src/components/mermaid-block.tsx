import { useEffect, useId, useRef, useState } from "react";

import { useResolvedTheme } from "@/components/theme-provider";

type Outcome = { svg: string } | { error: string };

export function CodeBox({ code }: { code: string }) {
  return (
    <pre className="max-w-full overflow-x-auto rounded-md bg-muted p-3 font-mono text-xs">
      <code>{code}</code>
    </pre>
  );
}

/**
 * A `mermaid` fenced block. The mermaid library is loaded on first use, with
 * `securityLevel: 'strict'` and the current light or dark theme; the diagram is redrawn when the
 * theme changes. A diagram that cannot be drawn shows as a plain code block with the error under
 * it in muted text.
 */
export function MermaidBlock({ code }: { code: string }) {
  const theme = useResolvedTheme();
  const renderId = `mermaid-${useId().replace(/[^a-zA-Z0-9_-]/g, "")}`;
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  const host = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let current = true;
    setOutcome(null);
    (async () => {
      try {
        // Dynamic on purpose: mermaid is large and the spec loads it at the first diagram, not at start.
        const { default: mermaid } = await import("mermaid");
        mermaid.initialize({ startOnLoad: false, securityLevel: "strict", theme: theme === "dark" ? "dark" : "default" });
        const { svg } = await mermaid.render(renderId, code);
        if (current) setOutcome({ svg });
      } catch (error) {
        // A failed render leaves its scratch element in the page.
        document.getElementById(`d${renderId}`)?.remove();
        if (current) setOutcome({ error: error instanceof Error ? error.message : String(error) });
      }
    })();
    return () => {
      current = false;
    };
  }, [code, theme, renderId]);

  useEffect(() => {
    if (!outcome || !("svg" in outcome) || !host.current) return;
    // The SVG was sanitised by mermaid (strict); it is parsed and adopted as nodes, never injected as a string.
    const parsed = new DOMParser().parseFromString(outcome.svg, "text/html");
    const svg = parsed.body.querySelector("svg");
    if (svg) host.current.replaceChildren(document.importNode(svg, true));
  }, [outcome]);

  if (outcome && "error" in outcome) {
    return (
      <div>
        <CodeBox code={code} />
        <p className="mt-1 text-xs text-muted-foreground">{outcome.error}</p>
      </div>
    );
  }
  if (!outcome) return <CodeBox code={code} />;
  return <div ref={host} role="img" aria-label="Diagram" className="max-w-full overflow-x-auto" />;
}
