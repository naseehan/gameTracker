import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { GameSession } from '../types';
import { formatPlaytime, formatRelativeTime } from '../utils/formatters';
import { useLibraryStore } from '../store/libraryStore';
import { GameCover } from '../components/GameCover';

export function RecentlyPlayed() {
  const [sessions, setSessions] = useState<GameSession[]>([]);
  const [loading, setLoading] = useState(true);
  const { games, fetchGames } = useLibraryStore();

  useEffect(() => {
    if (games.length === 0) fetchGames();
    
    const fetchRecent = async () => {
      try {
        const recent = await invoke<GameSession[]>('get_recent_sessions', { limit: 20 });
        setSessions(recent);
      } catch (err) {
        console.error(err);
      } finally {
        setLoading(false);
      }
    };
    
    fetchRecent();
  }, []);

  if (loading) {
    return <div className="p-8 text-gray-500">Loading recent sessions...</div>;
  }

  return (
    <div className="flex flex-col h-full">
      <header className="px-8 py-6 border-b border-gray-800/50 backdrop-blur-md sticky top-0 z-10">
        <h2 className="text-2xl font-bold text-white tracking-wide">Recently Played</h2>
      </header>

      <div className="flex-1 overflow-y-auto p-8">
        {sessions.length === 0 ? (
          <div className="text-gray-500 italic">No recent sessions found.</div>
        ) : (
          <div className="space-y-4 max-w-4xl">
            {sessions.map((session, i) => {
              const game = games.find(g => g.id === session.game_id);
              return (
                <div key={i} className="flex items-center justify-between bg-gray-900 border border-gray-800 p-4 rounded-xl">
                  <div className="flex items-center space-x-4">
                    <div className="w-12 h-16 rounded overflow-hidden flex-shrink-0">
                      <GameCover path={game?.cover_path ?? null} alt="" iconSize={20} className="w-full h-full" />
                    </div>
                    <div>
                      <h4 className="font-semibold text-gray-200">{game?.name || `Game ID: ${session.game_id}`}</h4>
                      <p className="text-sm text-gray-500">{formatRelativeTime(session.started_at)}</p>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="text-gray-300 font-medium">
                      {session.duration_seconds !== null ? formatPlaytime(session.duration_seconds) : <span className="text-green-500 animate-pulse">Running</span>}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
