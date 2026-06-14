// Pinned ("favorite") devices, persisted in localStorage. Keyed with the same
// "<scope>:<id>" convention as aliases (e.g. `bt:AA:BB:..`, `audio:<sink>`).
// Favorites sort to the top of their list.
import { useCallback, useState } from "react";

const KEY = "dmgr.favorites";

function load(): string[] {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) return JSON.parse(raw) as string[];
  } catch {
    /* ignore corrupt storage */
  }
  return [];
}

function persist(arr: string[]): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(arr));
  } catch {
    /* ignore */
  }
}

/** Reactive favorites for a component. `has(key)` / `toggle(key)`. */
export function useFavorites() {
  const [favs, setFavs] = useState<Set<string>>(() => new Set(load()));

  const toggle = useCallback((key: string) => {
    setFavs((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      persist([...next]);
      return next;
    });
  }, []);

  const has = useCallback((key: string) => favs.has(key), [favs]);

  return { has, toggle };
}
