import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ArrowLeft, Play, Clock, Calendar, Monitor } from 'lucide-react';
import { GameLibraryItem } from '../types';
import { GameCover } from '../components/GameCover';
import { useNavigationStore } from '../store/navigationStore';
import { PlaytimeChart } from '../components/statistics/PlaytimeChart';

interface GameStatistics {
  total_playtime_seconds: number;
  total_sessions: number;
  average_session_seconds: number;
  longest_session_seconds: number;
  first_played_at: string | null;
  last_played_at: string | null;
  played_today_seconds: number;
  played_week_seconds: number;
  played_month_seconds: number;
}

interface GameSessionDto {
  session_id: number;
  started_at: string;
  ended_at: string | null;
  duration_seconds: number;
  is_active: boolean;
}

interface DailyPlaytime {
  date_label: string;
  playtime_seconds: number;
}

function formatDuration(seconds: number): string {
  if (seconds === 0) return '0m';
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatShortDate(dateStr: string): string {
  const d = new Date(dateStr);
  return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
}

function formatTime(dateStr: string): string {
  const d = new Date(dateStr);
  return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
}

export function GameDetails({ gameId }: { gameId: number }) {
  const { goBack } = useNavigationStore();
  const [game, setGame] = useState<GameLibraryItem | null>(null);
  const [stats, setStats] = useState<GameStatistics | null>(null);
  const [sessions, setSessions] = useState<GameSessionDto[]>([]);
  const [history, setHistory] = useState<DailyPlaytime[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  
  const [timeRange, setTimeRange] = useState<'7' | '30'>('30');
  
  const fetchAll = async () => {
    try {
      const g = await invoke<GameLibraryItem>('get_game_details', { gameId });
      const s = await invoke<GameStatistics>('get_game_stats', { gameId });
      const h = await invoke<DailyPlaytime[]>('get_game_history', { gameId, days: parseInt(timeRange) });
      const sess = await invoke<GameSessionDto[]>('get_game_session_list', { gameId, limit: 10, offset: 0 });
      
      setGame(g);
      setStats(s);
      setHistory(h);
      setSessions(sess);
    } catch (e) {
      console.error(e);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchAll();
    const interval = setInterval(fetchAll, 5000); // Polling for active duration
    return () => clearInterval(interval);
  }, [gameId, timeRange]);

  if (isLoading || !game || !stats) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="animate-spin w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full"></div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col overflow-y-auto">
      {/* Header Banner */}
      <div className="relative h-64 bg-gray-900 border-b border-gray-800 flex-shrink-0">
        <div className="absolute inset-0 overflow-hidden">
          <GameCover path={game.cover_path} alt="" className="w-full h-full opacity-20 blur-sm" fallback={null} />
          <div className="absolute inset-0 bg-gradient-to-t from-gray-950 via-gray-900/60 to-transparent"></div>
        </div>
        
        <div className="absolute inset-0 p-8 flex items-end">
          <div className="flex items-end space-x-6">
            <div className="relative w-32 h-48 rounded-lg shadow-2xl overflow-hidden bg-gray-800 border border-gray-700 flex-shrink-0">
              <GameCover path={game.cover_path} alt={game.name} iconSize={32} className="w-full h-full" />
              <div 
                className="absolute inset-0 bg-black/60 flex items-center justify-center opacity-0 hover:opacity-100 transition-opacity cursor-pointer text-sm font-medium text-white"
                onClick={async () => {
                  try {
                    const { open } = await import('@tauri-apps/plugin-dialog');
                    const filePath = await open({
                      multiple: false,
                      directory: false,
                      filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }]
                    });
                    if (filePath) {
                      const coverPath = await invoke<string>('set_game_cover', { gameId: game.id, sourcePath: filePath });
                      setGame({ ...game, cover_path: coverPath });
                    }
                  } catch (e) {
                    console.error(e);
                  }
                }}
              >
                Change Cover
              </div>
            </div>
            
            <div className="pb-2">
              <button onClick={goBack} className="flex items-center text-gray-400 hover:text-white transition-colors text-sm mb-4">
                <ArrowLeft size={16} className="mr-1" /> Back
              </button>
              
              <div className="flex items-center space-x-3 mb-2">
                <h1 className="text-4xl font-bold text-white tracking-tight">{game.name}</h1>
                {game.currently_playing && (
                  <span className="px-3 py-1 bg-green-500/20 text-green-400 text-xs font-bold rounded-full border border-green-500/30 flex items-center shadow-lg shadow-green-500/20">
                    <span className="w-2 h-2 rounded-full bg-green-400 mr-2 animate-pulse"></span>
                    PLAYING
                  </span>
                )}
                {game.is_ignored && (
                  <span className="px-3 py-1 bg-gray-800 text-gray-400 text-xs font-bold rounded-full border border-gray-700">
                    IGNORED
                  </span>
                )}
              </div>
              
              <div className="flex items-center space-x-3 mb-4">
                <button 
                  onClick={async () => {
                    try {
                      await invoke('set_game_ignored', { gameId: game.id, isIgnored: !game.is_ignored });
                      setGame({ ...game, is_ignored: !game.is_ignored });
                    } catch (e) {
                      console.error(e);
                    }
                  }}
                  className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors border ${game.is_ignored ? "bg-indigo-600 border-indigo-500 text-white hover:bg-indigo-500" : "bg-gray-800 border-gray-700 text-gray-300 hover:bg-gray-700 hover:text-white"}`}
                >
                  {game.is_ignored ? 'Unignore' : 'Ignore'}
                </button>
                <button 
                  onClick={async () => {
                    if (confirm(`Remove "${game.name}" from library?`)) {
                      try {
                        await invoke('remove_game_from_library', { gameId: game.id, deleteHistory: false });
                        goBack();
                      } catch (e) {
                        console.error(e);
                      }
                    }
                  }}
                  className="px-3 py-1.5 rounded-lg text-xs font-medium transition-colors border border-red-900 bg-red-950/30 text-red-400 hover:bg-red-900/50 hover:text-red-300"
                >
                  Remove
                </button>
              </div>
              <div className="flex items-center space-x-4 text-sm text-gray-300">
                <div className="flex items-center">
                  <Monitor size={14} className="mr-1 text-gray-500" />
                  {game.platforms && game.platforms.length > 0 ? game.platforms.join(', ') : 'Local'}
                </div>
                <div className="flex items-center">
                  <span className={game.current_install_status === 'Installed' ? 'text-indigo-400 font-medium' : 'text-gray-500'}>
                    {game.current_install_status}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="p-8 max-w-6xl mx-auto w-full space-y-8">
        
        {/* Insights Bar */}
        {stats.total_playtime_seconds > 0 && (
          <div className="flex flex-wrap gap-4">
            <div className="bg-indigo-500/10 border border-indigo-500/20 text-indigo-200 px-4 py-3 rounded-xl text-sm flex-1 min-w-[200px]">
              You've played <strong className="text-white">{formatDuration(stats.total_playtime_seconds)}</strong> in total.
            </div>
            {stats.average_session_seconds > 0 && (
              <div className="bg-purple-500/10 border border-purple-500/20 text-purple-200 px-4 py-3 rounded-xl text-sm flex-1 min-w-[200px]">
                Your average session is <strong className="text-white">{formatDuration(stats.average_session_seconds)}</strong>.
              </div>
            )}
            {stats.longest_session_seconds > 0 && (
              <div className="bg-blue-500/10 border border-blue-500/20 text-blue-200 px-4 py-3 rounded-xl text-sm flex-1 min-w-[200px]">
                Longest session: <strong className="text-white">{formatDuration(stats.longest_session_seconds)}</strong>.
              </div>
            )}
          </div>
        )}

        {/* Stats Grid */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-gray-900 border border-gray-800 rounded-xl p-5 shadow-lg">
            <div className="flex items-center text-gray-400 mb-2">
              <Clock size={16} className="mr-2 text-indigo-400" />
              <span className="text-sm font-medium uppercase tracking-wider">Total Playtime</span>
            </div>
            <div className="text-3xl font-bold text-white">{formatDuration(stats.total_playtime_seconds)}</div>
          </div>
          
          <div className="bg-gray-900 border border-gray-800 rounded-xl p-5 shadow-lg">
            <div className="flex items-center text-gray-400 mb-2">
              <Play size={16} className="mr-2 text-indigo-400" />
              <span className="text-sm font-medium uppercase tracking-wider">Sessions</span>
            </div>
            <div className="text-3xl font-bold text-white">{stats.total_sessions}</div>
          </div>

          <div className="bg-gray-900 border border-gray-800 rounded-xl p-5 shadow-lg">
            <div className="flex items-center text-gray-400 mb-2">
              <Calendar size={16} className="mr-2 text-indigo-400" />
              <span className="text-sm font-medium uppercase tracking-wider">Played Today</span>
            </div>
            <div className="text-3xl font-bold text-white">{formatDuration(stats.played_today_seconds)}</div>
          </div>

          <div className="bg-gray-900 border border-gray-800 rounded-xl p-5 shadow-lg">
            <div className="flex items-center text-gray-400 mb-2">
              <Calendar size={16} className="mr-2 text-indigo-400" />
              <span className="text-sm font-medium uppercase tracking-wider">Played This Week</span>
            </div>
            <div className="text-3xl font-bold text-white">{formatDuration(stats.played_week_seconds)}</div>
          </div>
        </div>

        {/* Chart & History Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          
          <div className="lg:col-span-2 space-y-6">
            <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 shadow-lg">
              <div className="flex items-center justify-between mb-6">
                <h2 className="text-lg font-bold text-white">Playtime History</h2>
                <div className="flex bg-gray-800 rounded-lg p-1">
                  <button 
                    onClick={() => setTimeRange('7')}
                    className={`px-3 py-1 rounded text-xs font-medium transition-colors ${timeRange === '7' ? 'bg-indigo-600 text-white shadow-sm' : 'text-gray-400 hover:text-white'}`}
                  >
                    7D
                  </button>
                  <button 
                    onClick={() => setTimeRange('30')}
                    className={`px-3 py-1 rounded text-xs font-medium transition-colors ${timeRange === '30' ? 'bg-indigo-600 text-white shadow-sm' : 'text-gray-400 hover:text-white'}`}
                  >
                    30D
                  </button>
                </div>
              </div>
              <div className="h-64">
                <PlaytimeChart data={history} />
              </div>
            </div>
            
            {game.installation_paths && game.installation_paths.length > 0 && (
              <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 shadow-lg">
                <h2 className="text-lg font-bold text-white mb-4">Installations</h2>
                <div className="space-y-2">
                  {game.installation_paths.map((p, i) => (
                    <div key={i} className="text-sm text-gray-300 font-mono break-all bg-gray-800/50 p-3 rounded-lg border border-gray-700/50">
                      {p}
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>

          <div className="space-y-6">
            <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 shadow-lg flex flex-col h-full max-h-[500px]">
              <h2 className="text-lg font-bold text-white mb-4 flex-shrink-0">Session History</h2>
              
              <div className="flex-1 overflow-y-auto pr-2 space-y-3">
                {sessions.length === 0 ? (
                  <div className="text-gray-500 text-center py-8 text-sm">No sessions recorded yet.</div>
                ) : (
                  sessions.map(s => (
                    <div key={s.session_id} className={`p-3 rounded-lg border ${s.is_active ? 'bg-indigo-500/10 border-indigo-500/30' : 'bg-gray-800/50 border-gray-800'}`}>
                      <div className="flex justify-between items-start mb-1">
                        <span className="font-medium text-gray-200 text-sm">
                          {formatShortDate(s.started_at)}
                        </span>
                        <span className={`text-sm font-bold ${s.is_active ? 'text-indigo-400 animate-pulse' : 'text-gray-400'}`}>
                          {formatDuration(s.duration_seconds)}
                        </span>
                      </div>
                      <div className="flex justify-between items-center text-xs text-gray-500">
                        <span>{formatTime(s.started_at)} {s.ended_at ? `- ${formatTime(s.ended_at)}` : ''}</span>
                        {s.is_active && <span className="text-indigo-500 font-medium">Playing Now</span>}
                      </div>
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>
          
        </div>
      </div>
    </div>
  );
}