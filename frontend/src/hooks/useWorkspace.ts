import { useState, useCallback } from 'react';

const STORAGE_KEY_CURRENT = 'robocaucus:workspace:current';
const STORAGE_KEY_RECENT = 'robocaucus:workspace:recent';
const MAX_RECENT = 10;

function readStorage<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    if (raw === null) return fallback;
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

function writeStorage<T>(key: string, value: T): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Quota exceeded or unavailable — silently ignore
  }
}

export interface UseWorkspaceReturn {
  currentWorkspace: string | null;
  recentWorkspaces: string[];
  setWorkspace: (path: string) => void;
  clearWorkspace: () => void;
}

export function useWorkspace(): UseWorkspaceReturn {
  const [currentWorkspace, setCurrentWorkspace] = useState<string | null>(() =>
    readStorage<string | null>(STORAGE_KEY_CURRENT, null),
  );

  const [recentWorkspaces, setRecentWorkspaces] = useState<string[]>(() =>
    readStorage<string[]>(STORAGE_KEY_RECENT, []),
  );

  const setWorkspace = useCallback((path: string) => {
    setCurrentWorkspace(path);
    writeStorage(STORAGE_KEY_CURRENT, path);

    setRecentWorkspaces((prev) => {
      const filtered = prev.filter((p) => p !== path);
      const next = [path, ...filtered].slice(0, MAX_RECENT);
      writeStorage(STORAGE_KEY_RECENT, next);
      return next;
    });
  }, []);

  const clearWorkspace = useCallback(() => {
    setCurrentWorkspace(null);
    writeStorage(STORAGE_KEY_CURRENT, null);
  }, []);

  return { currentWorkspace, recentWorkspaces, setWorkspace, clearWorkspace };
}
