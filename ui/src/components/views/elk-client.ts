import ELK from "elkjs/lib/elk-api";
import ElkWorker from "elkjs/lib/elk-worker.min.js?worker";

import type { LayoutEngine } from "@/components/views/diagram-layout";

let elk: InstanceType<typeof ELK> | null = null;

/** The layered layout of the Diagram: elkjs in a Web Worker, started with the first level drawn. */
export const layoutInWorker: LayoutEngine = (graph) => {
  elk ??= new ELK({ workerFactory: () => new ElkWorker() });
  return elk.layout(graph);
};
