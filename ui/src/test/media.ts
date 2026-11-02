// A controllable `window.matchMedia` for tests: the viewport width, the system colour scheme and the
// reduced-motion preference are set by the test, and the listeners of the queries that changed are
// told, as a browser does.

interface Environment {
  width: number;
  dark: boolean;
  reducedMotion: boolean;
}

const environment: Environment = { width: 1280, dark: false, reducedMotion: false };

interface Entry {
  query: string;
  matches: boolean;
  listeners: Set<(event: MediaQueryListEvent) => void>;
}

const entries = new Set<Entry>();

/** Evaluates the few media features Weaver's UI uses. */
function evaluate(query: string): boolean {
  const q = query.toLowerCase();
  const width = /\((max|min)-width:\s*([\d.]+)px\)/.exec(q);
  if (width) {
    const limit = Number(width[2]);
    return width[1] === "max" ? environment.width <= limit : environment.width >= limit;
  }
  if (q.includes("prefers-color-scheme: dark")) return environment.dark;
  if (q.includes("prefers-color-scheme: light")) return !environment.dark;
  if (q.includes("prefers-reduced-motion: reduce")) return environment.reducedMotion;
  return false;
}

function notify(): void {
  for (const entry of entries) {
    const matches = evaluate(entry.query);
    if (matches === entry.matches) continue;
    entry.matches = matches;
    const event = { matches, media: entry.query } as MediaQueryListEvent;
    for (const listener of [...entry.listeners]) listener(event);
  }
}

export function installMatchMedia(): void {
  window.matchMedia = (query: string): MediaQueryList => {
    const entry: Entry = { query, matches: evaluate(query), listeners: new Set() };
    entries.add(entry);
    return {
      get matches() {
        return entry.matches;
      },
      media: query,
      onchange: null,
      addEventListener: (_type: string, listener: EventListenerOrEventListenerObject) =>
        entry.listeners.add(listener as (event: MediaQueryListEvent) => void),
      removeEventListener: (_type: string, listener: EventListenerOrEventListenerObject) => {
        entry.listeners.delete(listener as (event: MediaQueryListEvent) => void);
      },
      addListener: (listener: (event: MediaQueryListEvent) => void) => entry.listeners.add(listener),
      removeListener: (listener: (event: MediaQueryListEvent) => void) => entry.listeners.delete(listener),
      dispatchEvent: () => false,
    } as MediaQueryList;
  };
}

/** Back to a 1280 px wide window, light system, motion allowed; forgets every query. */
export function resetMedia(): void {
  environment.width = 1280;
  environment.dark = false;
  environment.reducedMotion = false;
  entries.clear();
}

export function setViewportWidth(width: number): void {
  environment.width = width;
  notify();
}

export function setSystemDark(dark: boolean): void {
  environment.dark = dark;
  notify();
}

export function setReducedMotion(reduced: boolean): void {
  environment.reducedMotion = reduced;
  notify();
}
