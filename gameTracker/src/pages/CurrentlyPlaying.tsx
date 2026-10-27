import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useLibraryStore } from '../store/libraryStore';
import { useNavigationStore } from '../store/navigationStore';
import { Gamepad2, Clock, Play } from 'lucide-react';

interface ActiveSession {
  session_id: number;
  game_id: number;
  started_at: string;
}

function LiveTimer({ startedAt }: { startedAt: string }) {
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    const start = new Date(startedAt).getTime();
    
    const updateTimer = () => {
      const now = Date.now();
      setElapsed(Math.max(0, Math.floor((now - start) / 1000)));
    };
    
    updateTimer();
    const interval = setInterval(updateTimer, 1000);
    return () => clearInterval(interval);
  }, [startedAt]);

  const h = Math.floor(elapsed / 3600);
  const m = Math.floor((elapsed % 3600) / 60);
  const s = elapsed % 60;
  
  const pad = (n: number) => n.toString().padStart(2, '0');

  return (
    <span className="font-mono font-bold tracking-wider">
      {h > 0 ? `${pad(h)}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`}
    </span>
  );
}

export function CurrentlyPlaying() {
  const { games, fetchGames } = useLibraryStore();
  const { navigate } = useNavigationStore();
  const [sessions, setSessions] = useState<ActiveSession[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  const fetchSessions = async () => {
    try {
      const active = await invoke<ActiveSession[]>('get_active_sessions');
      setSessions(active);
      setIsLoading(false);
    } catch (e) {
      console.error(e);
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchGames(); // ensure games are loaded
    fetchSessions();
    const interval = setInterval(fetchSessions, 2000); // Poll active sessions quickly
    return () => clearInterval(interval);
  }, []);

  if (isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center bg-gray-950">
        <div className="animate-spin w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full"></div>
      </div>
    );
  }

  if (sessions.length === 0) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center bg-gray-950 p-8">
        <div className="w-24 h-24 bg-gray-900 rounded-full flex items-center justify-center mb-6 border border-gray-800 shadow-xl">
          <Gamepad2 size={40} className="text-gray-600" />
        </div>
        <h2 className="text-2xl font-bold text-white mb-2">Nothing playing right now</h2>
        <p className="text-gray-400 mb-8 max-w-sm text-center">Launch a game and it will appear here automatically.</p>
        <button 
          onClick={() => navigate({ type: 'library' })}
          className="px-6 py-3 bg-indigo-600 hover:bg-indigo-700 text-white rounded-xl font-medium transition-colors shadow-lg shadow-indigo-500/20"
        >
          Browse Library
        </button>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col bg-gray-950 overflow-y-auto p-8">
      <h1 className="text-3xl font-bold text-white mb-8 tracking-tight">Currently Playing</h1>
      
      <div className="space-y-6 max-w-5xl">
        {sessions.map(session => {
          const game = games.find(g => g.id === session.game_id);
          if (!game) return null;
          
          const startTime = new Date(session.started_at).toLocaleTimeString(undefined, { 
            hour: 'numeric', minute: '2-digit' 
          });

          return (
            <div key={session.session_id} className="relative bg-gray-900 rounded-2xl border border-gray-800 overflow-hidden shadow-2xl flex flex-col md:flex-row group transition-colors hover:border-indigo-500/50 focus-within:ring-2 focus-within:ring-indigo-500">
              {/* Cover */}
              <div className="w-full md:w-64 h-64 md:h-auto flex-shrink-0 relative bg-gray-800">
                {game.cover_path ? (
                  <img 
                    src={`http://localhost:1420/__tauri_asset__${encodeURIComponent(game.cover_path)}`}
                    className="w-full h-full object-cover"
                    alt={game.name}
                  />
                ) : (
                  <div className="w-full h-full flex items-center justify-center">
                    <Gamepad2 size={48} className="text-gray-600" />
                  </div>
                )}
                {/* Gradient overlay for text readability on small screens */}
                <div className="absolute inset-0 bg-gradient-to-t from-gray-950 via-transparent to-transparent md:hidden"></div>
              </div>

              {/* Content */}
              <div className="p-8 flex flex-col justify-between flex-1">
                <div>
                  <div className="flex items-center space-x-3 mb-2">
                    <span className="px-3 py-1 bg-indigo-500/20 text-indigo-400 text-xs font-bold rounded-full border border-indigo-500/30 flex items-center shadow-lg shadow-indigo-500/20">
                      <span className="w-2 h-2 rounded-full bg-indigo-400 mr-2 animate-pulse"></span>
                      PLAYING NOW
                    </span>
                    {game.platforms && game.platforms.length > 0 && (
                      <span className="text-xs text-gray-400 font-medium bg-gray-800 px-2 py-1 rounded">
                        {game.platforms.join(', ')}
                      </span>
                    )}
                  </div>
                  
                  <h2 className="text-3xl font-bold text-white mb-6 tracking-tight">{game.name}</h2>
                  
                  <div className="space-y-4">
                    <div className="flex items-center text-gray-300">
                      <div className="w-10 h-10 rounded-full bg-gray-800 flex items-center justify-center mr-4 border border-gray-700">
                        <Play size={18} className="text-indigo-400 ml-1" />
                      </div>
                      <div>
                        <p className="text-xs text-gray-500 font-medium uppercase tracking-wider mb-0.5">Session Duration</p>
                        <p className="text-xl text-white">
                          <LiveTimer startedAt={session.started_at} />
                        </p>
                      </div>
                    </div>
                    
                    <div className="flex items-center text-gray-300">
                      <div className="w-10 h-10 rounded-full bg-gray-800 flex items-center justify-center mr-4 border border-gray-700">
                        <Clock size={18} className="text-indigo-400" />
                      </div>
                      <div>
                        <p className="text-xs text-gray-500 font-medium uppercase tracking-wider mb-0.5">Started At</p>
                        <p className="text-lg font-medium text-white">{startTime}</p>
                      </div>
                    </div>
                  </div>
                </div>

                <div className="mt-8 pt-6 border-t border-gray-800/50">
                  <button 
                    onClick={() => navigate({ type: 'game_details', gameId: game.id })}
                    className="inline-flex items-center px-5 py-2.5 bg-gray-800 hover:bg-gray-700 text-white rounded-lg font-medium transition-colors border border-gray-700 hover:border-gray-600"
                  >
                    View Game Details
                  </button>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}