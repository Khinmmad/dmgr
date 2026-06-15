// Per-device custom names ("aliases"), persisted in localStorage and keyed by a
// stable "<scope>:<id>" string, e.g. `bt:AA:BB:CC:DD:EE:FF` or `audio:<sink>`.
// Reusable across panels via the useAliases() hook.
import { useCallback, useState } from "react";

const KEY = "dmgr.aliases";
type AliasMap = Record<string, string>;

function load(): AliasMap {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) return JSON.parse(raw) as AliasMap;
  } catch {
    /* ignore corrupt storage */
  }
  return {};
}

function persist(map: AliasMap): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(map));
  } catch {
    /* ignore */
  }
}

/** Reactive access to the alias map for a component. */
export function useAliases() {
  const [map, setMap] = useState<AliasMap>(load);

  /** Set, or (with an empty/whitespace value) clear, the alias for `key`. */
  const rename = useCallback((key: string, alias: string) => {
    setMap((prev) => {
      const next = { ...prev };
      const v = alias.trim();
      if (v) next[key] = v;
      else delete next[key];
      persist(next);
      return next;
    });
  }, []);

  /** Alias if set, else the given fallback. */
  const name = useCallback((key: string, fallback: string) => map[key] ?? fallback, [map]);

  return { map, name, rename };
}
