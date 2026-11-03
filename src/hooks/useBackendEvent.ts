import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useLibraryStore } from '../store/libraryStore';
import { useSessionsStore } from '../store/sessionsStore';
import { reportError } from '../utils/errors';

/** Events the backend emits when data changes (see src-tauri/src/events.rs). */
export type BackendEvent = 'library-changed' | 'sessions-changed';

/** Calls `handler` whenever any of the given backend events fires. */
export function useBackendEvent(events: BackendEvent[], handler: () => void) {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;
  const key = events.join(',');

  useEffect(() => {
    const unlistens = key.split(',').map((event) => listen(event, () => handlerRef.current()));
    return () => {
      unlistens.forEach((p) => p.then((unlisten) => unlisten()).catch(() => {}));
    };
  }, [key]);
}

/** Loads the library and running sessions, and reloads them when the backend says they changed. */
export function useBackendSync() {
  const fetchGames = useLibraryStore((s) => s.fetchGames);
  const fetchActive = useSessionsStore((s) => s.fetchActive);

  const refreshSessions = () => fetchActive().catch((e) => reportError("Couldn't load running games", e));

  useEffect(() => {
    fetchGames();
    refreshSessions();
  }, []);

  // The library shows which games are being played, and running sessions show game
  // names and covers, so both are reloaded for either event.
  useBackendEvent(['library-changed', 'sessions-changed'], () => {
    fetchGames();
    refreshSessions();
  });
}
