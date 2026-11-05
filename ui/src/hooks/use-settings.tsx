import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import type { SettingChange } from "@/bindings/SettingChange";
import type { SettingKey } from "@/bindings/SettingKey";
import type { Settings } from "@/bindings/Settings";
import type { SettingsSnapshot } from "@/bindings/SettingsSnapshot";
import { call, isAppError, on } from "@/lib/backend";

export interface SettingsContextValue {
  /** The settings in force; `null` until the backend answered. */
  snapshot: SettingsSnapshot | null;
  /** Why the first load failed. */
  loadError: string | null;
  /** Asks again after a failed load. */
  reload: () => void;
  /** Sends one change. Resolves when stored; rejects with an `AppError` (the backend's refusal or failure). */
  save: (change: SettingChange) => Promise<void>;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

/**
 * The settings of the app as the backend holds them. The UI keeps no copy of its own: a snapshot
 * arrives from `settings_get`, from the answer to `settings_set` and from the `settings-changed`
 * event (a change made in any window), and the one with the highest revision wins.
 */
export function SettingsProvider({ children }: { children: ReactNode }) {
  const [snapshot, setSnapshot] = useState<SettingsSnapshot | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const revision = useRef(-1);

  const accept = useCallback((incoming: SettingsSnapshot) => {
    if (incoming.revision < revision.current) return;
    revision.current = incoming.revision;
    setSnapshot(incoming);
    setLoadError(null);
  }, []);

  useEffect(() => {
    let current = true;
    // Listen first, then ask: a change between the two is not lost.
    const unlisten = on("settings-changed", (incoming) => current && accept(incoming));
    call("settings_get")
      .then((known) => current && accept(known))
      .catch((error: unknown) => {
        if (current) setLoadError(isAppError(error) ? error.message : "The settings could not be read.");
      });
    return () => {
      current = false;
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, [accept, attempt]);

  const save = useCallback(
    async (change: SettingChange) => {
      accept(await call("settings_set", { change }));
    },
    [accept],
  );
  const reload = useCallback(() => setAttempt((n) => n + 1), []);

  const value = useMemo(() => ({ snapshot, loadError, reload, save }), [snapshot, loadError, reload, save]);
  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}

export function useSettings(): SettingsContextValue {
  const value = useContext(SettingsContext);
  if (!value) throw new Error("useSettings needs a SettingsProvider");
  return value;
}

/** The value of `key` as the text the backend exchanges (a whole number as its digits). */
export function settingText(settings: Settings, key: SettingKey): string {
  switch (key) {
    case "ompPath":
      return settings.ompPath;
    case "models.analysis":
      return settings.models.analysis;
    case "models.scan":
      return settings.models.scan;
    case "models.execution":
      return settings.models.execution;
    case "maxParallelAgents":
      return String(settings.maxParallelAgents);
    case "language":
      return settings.language;
    case "theme":
      return settings.theme;
    case "editor.preset":
      return settings.editor.preset;
    case "editor.command":
      return settings.editor.command;
    case "permissions":
      return settings.permissions;
    case "systemCertificates.enabled":
      return String(settings.systemCertificates.enabled);
  }
}
