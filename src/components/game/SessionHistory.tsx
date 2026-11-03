import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Pencil, Trash2 } from 'lucide-react';
import { GameSessionDto } from '../../types';
import { formatPlaytime, formatShortDate, formatTime } from '../../utils/formatters';
import { reportError } from '../../utils/errors';
import { confirmAction } from '../../store/confirmStore';
import { toast } from '../../store/toastStore';
import { useBackendEvent } from '../../hooks/useBackendEvent';
import { LiveTimer } from '../LiveTimer';
import { EditSessionModal } from './EditSessionModal';

const PAGE_SIZE = 20;

export function SessionHistory({ gameId }: { gameId: number }) {
  const [sessions, setSessions] = useState<GameSessionDto[]>([]);
  const [hasMore, setHasMore] = useState(false);
  const [editing, setEditing] = useState<GameSessionDto | null>(null);

  // Reloads as many sessions as are shown now, so "Load more" pages stay loaded.
  const load = async (count: number) => {
    try {
      const list = await invoke<GameSessionDto[]>('get_game_session_list', { gameId, limit: count + 1, offset: 0 });
      const finished = list.filter((s) => !s.is_active);
      setHasMore(finished.length > count);
      const active = list.filter((s) => s.is_active);
      setSessions([...active, ...finished.slice(0, count)]);
    } catch (err) {
      reportError("Couldn't load sessions", err);
    }
  };
  const shownFinished = () => Math.max(PAGE_SIZE, sessions.filter((s) => !s.is_active).length);

  useEffect(() => {
    load(PAGE_SIZE);
  }, [gameId]);
  useBackendEvent(['sessions-changed'], () => load(shownFinished()));

  const remove = async (session: GameSessionDto) => {
    const confirmed = await confirmAction({
      title: 'Delete session?',
      message: `${formatShortDate(session.started_at)}, ${formatTime(session.started_at)} – ${session.ended_at ? formatTime(session.ended_at) : ''} (${formatPlaytime(session.duration_seconds)}) will be removed from your play history.`,
      confirmLabel: 'Delete session',
      danger: true,
    });
    if (!confirmed) return;
    try {
      await invoke('delete_session', { sessionId: session.session_id });
      toast.success('Session deleted.');
    } catch (err) {
      reportError("Couldn't delete the session", err);
    }
  };

  return (
    <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 shadow-lg flex flex-col max-h-[640px]">
      <h2 className="text-lg font-bold text-white mb-1 flex-shrink-0">Session History</h2>
      <p className="text-xs text-gray-500 mb-4">Fix a session that was recorded wrong, or delete it.</p>

      <div className="flex-1 overflow-y-auto pr-1 space-y-2">
        {sessions.length === 0 ? (
          <div className="text-gray-500 text-center py-8 text-sm">No sessions recorded yet.</div>
        ) : (
          sessions.map((s) => (
            <div
              key={s.session_id}
              className={`group p-3 rounded-lg border ${s.is_active ? 'bg-indigo-500/10 border-indigo-500/30' : 'bg-gray-800/50 border-gray-800'}`}
            >
              <div className="flex justify-between items-start mb-1 gap-2">
                <span className="font-medium text-gray-200 text-sm">{formatShortDate(s.started_at)}</span>
                {s.is_active ? (
                  <LiveTimer startedAt={s.started_at} className="text-sm font-bold text-indigo-400" />
                ) : (
                  <span className="text-sm font-bold text-gray-400">{formatPlaytime(s.duration_seconds)}</span>
                )}
              </div>
              <div className="flex justify-between items-center text-xs text-gray-500">
                <span>{formatTime(s.started_at)} {s.ended_at ? `– ${formatTime(s.ended_at)}` : ''}</span>
                {s.is_active ? (
                  <span className="text-indigo-400 font-medium">Playing now</span>
                ) : (
                  <span className="flex gap-1 opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity">
                    <button onClick={() => setEditing(s)} className="p-1 rounded hover:bg-gray-700 hover:text-white" aria-label="Edit session">
                      <Pencil size={14} />
                    </button>
                    <button onClick={() => remove(s)} className="p-1 rounded hover:bg-red-900/50 hover:text-red-300" aria-label="Delete session">
                      <Trash2 size={14} />
                    </button>
                  </span>
                )}
              </div>
            </div>
          ))
        )}
        {hasMore && (
          <button
            onClick={() => load(shownFinished() + PAGE_SIZE)}
            className="w-full py-2 text-sm font-medium text-indigo-400 hover:text-indigo-300"
          >
            Load more
          </button>
        )}
      </div>

      {editing && <EditSessionModal session={editing} onClose={() => setEditing(null)} />}
    </div>
  );
}
