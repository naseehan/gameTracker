import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ArrowLeft, Play, Clock, Calendar, Monitor, Trophy, MoreHorizontal, Pencil, Image, Trash2, Eye, EyeOff } from 'lucide-react';
import { DailyPlaytime, GameLibraryItem, GameStatistics } from '../types';
import { useNavigationStore } from '../store/navigationStore';
import { PlaytimeChart } from '../components/statistics/PlaytimeChart';
import { GameCover } from '../components/GameCover';
import { SessionHistory } from '../components/game/SessionHistory';
import { DetectionCard } from '../components/game/DetectionCard';
import { useBackendEvent } from '../hooks/useBackendEvent';
import { confirmWithOption } from '../store/confirmStore';
import { toast } from '../store/toastStore';
import { formatPlaytime, formatShortDate } from '../utils/formatters';
import { describeError, reportError } from '../utils/errors';

export function GameDetails({ gameId }: { gameId: number }) {
  const { goBack } = useNavigationStore();
  const [game, setGame] = useState<GameLibraryItem | null>(null);
  const [stats, setStats] = useState<GameStatistics | null>(null);
  const [history, setHistory] = useState<DailyPlaytime[]>([]);
  const [notFound, setNotFound] = useState<string | null>(null);
  const [timeRange, setTimeRange] = useState<'7' | '30'>('30');

  const fetchAll = async () => {
    try {
      const [g, s, h] = await Promise.all([
        invoke<GameLibraryItem>('get_game_details', { gameId }),
        invoke<GameStatistics>('get_game_stats', { gameId }),
        invoke<DailyPlaytime[]>('get_game_history', { gameId, days: parseInt(timeRange) }),
      ]);
      setGame(g);
      setStats(s);
      setHistory(h);
      setNotFound(null);
    } catch (e) {
      setNotFound(describeError(e));
    }
  };

  useEffect(() => {
    fetchAll();
  }, [gameId, timeRange]);
  useBackendEvent(['sessions-changed', 'library-changed'], fetchAll);

  // While the game is running its totals grow; refresh them every minute.
  useEffect(() => {
    if (!game?.currently_playing) return;
    const interval = setInterval(fetchAll, 60_000);
    return () => clearInterval(interval);
  }, [game?.currently_playing, timeRange]);

  if (notFound) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center p-8 text-center">
        <p className="text-gray-300 mb-4">{notFound}</p>
        <button onClick={goBack} className="px-5 py-2.5 bg-gray-800 hover:bg-gray-700 border border-gray-700 rounded-lg text-sm font-medium">
          Go back
        </button>
      </div>
    );
  }

  if (!game || !stats) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="animate-spin w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full" aria-label="Loading"></div>
      </div>
    );
  }

  const changeCover = () => {
    invoke<string | null>('choose_game_cover', { gameId: game.id })
      .then((cover) => cover && setGame({ ...game, cover_path: cover }))
      .catch((e) => reportError("Couldn't change the cover", e));
  };

  const toggleTracking = async () => {
    try {
      await invoke('set_game_ignored', { gameId: game.id, isIgnored: !game.is_ignored });
      setGame({ ...game, is_ignored: !game.is_ignored });
    } catch (e) {
      reportError("Couldn't change tracking", e);
    }
  };

  const remove = async () => {
    const { confirmed, checked } = await confirmWithOption({
      title: `Remove ${game.name}?`,
      message: 'It disappears from your library and stops being tracked. You can restore it from Settings.',
      confirmLabel: 'Remove',
      danger: true,
      checkboxLabel: 'Also delete its play history (this cannot be undone)',
    });
    if (!confirmed) return;
    try {
      await invoke('remove_game_from_library', { gameId: game.id, deleteHistory: checked });
      toast.success(`Removed ${game.name}.`);
      goBack();
    } catch (e) {
      reportError("Couldn't remove the game", e);
    }
  };

  return (
    <div className="flex-1 flex flex-col overflow-y-auto">
      {/* Header Banner */}
      <div className="relative bg-gray-900 border-b border-gray-800 flex-shrink-0">
        <div className="absolute inset-0 overflow-hidden">
          <GameCover path={game.cover_path} alt="" className="w-full h-full opacity-20 blur-sm" fallback={null} />
          <div className="absolute inset-0 bg-gradient-to-t from-gray-950 via-gray-900/60 to-transparent"></div>
        </div>

        <div className="relative p-8 pt-6">
          <button onClick={goBack} className="flex items-center text-gray-400 hover:text-white transition-colors text-sm mb-6">
            <ArrowLeft size={16} className="mr-1" /> Back
          </button>
          <div className="flex items-end gap-6">
            <button
              onClick={changeCover}
              className="group relative w-32 h-48 rounded-lg shadow-2xl overflow-hidden bg-gray-800 border border-gray-700 flex-shrink-0"
              aria-label="Change cover"
            >
              <GameCover path={game.cover_path} alt={game.name} iconSize={32} className="w-full h-full" />
              <span className="absolute inset-0 bg-black/60 flex items-center justify-center opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100 transition-opacity text-sm font-medium text-white">
                Change cover
              </span>
            </button>

            <div className="pb-1 min-w-0 flex-1">
              <div className="flex items-center gap-3 mb-3 flex-wrap">
                <GameTitle game={game} onRenamed={(name) => setGame({ ...game, name })} />
                {game.currently_playing && (
                  <span className="px-3 py-1 bg-green-500/20 text-green-400 text-xs font-bold rounded-full border border-green-500/30 flex items-center">
                    <span className="w-2 h-2 rounded-full bg-green-400 mr-2 animate-pulse"></span>
                    PLAYING
                  </span>
                )}
                {game.is_ignored && (
                  <span className="px-3 py-1 bg-gray-800 text-gray-400 text-xs font-bold rounded-full border border-gray-700">
                    NOT TRACKED
                  </span>
                )}
              </div>
              <div className="flex items-center gap-4 text-sm text-gray-300 flex-wrap">
                <span className="flex items-center">
                  <Monitor size={14} className="mr-1 text-gray-500" />
                  {game.platforms.length > 0 ? game.platforms.join(', ') : 'Added by you'}
                </span>
                <span className={game.current_install_status === 'Not Installed' ? 'text-gray-500' : 'text-indigo-400 font-medium'}>
                  {game.current_install_status === 'Not Installed' ? 'Not installed' : 'Installed'}
                </span>
                {stats.first_played_at && <span className="text-gray-500">First played {formatShortDate(stats.first_played_at)}</span>}
              </div>
            </div>

            <div className="flex items-center gap-2 pb-1 flex-shrink-0">
              <button
                onClick={toggleTracking}
                className="flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium border bg-gray-800 border-gray-700 text-gray-200 hover:bg-gray-700"
                title={game.is_ignored ? 'Record playtime for this game again' : 'Stop recording playtime for this game'}
              >
                {game.is_ignored ? <Eye size={16} /> : <EyeOff size={16} />}
                {game.is_ignored ? 'Track again' : "Don't track"}
              </button>
              <ActionsMenu onChangeCover={changeCover} onRemove={remove} />
            </div>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="p-8 max-w-6xl mx-auto w-full space-y-8">
        <div className="grid grid-cols-2 xl:grid-cols-4 gap-4">
          <Stat icon={<Clock size={16} />} label="Total playtime" value={formatPlaytime(stats.total_playtime_seconds)} />
          <Stat
            icon={<Play size={16} />}
            label="Sessions"
            value={String(stats.total_sessions)}
            detail={stats.total_sessions > 0 ? `${formatPlaytime(stats.average_session_seconds)} on average` : undefined}
          />
          <Stat
            icon={<Calendar size={16} />}
            label="Last 7 days"
            value={formatPlaytime(stats.played_week_seconds)}
            detail={`Today: ${formatPlaytime(stats.played_today_seconds)}`}
          />
          <Stat icon={<Trophy size={16} />} label="Longest session" value={formatPlaytime(stats.longest_session_seconds)} />
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          <div className="lg:col-span-2 space-y-6">
            <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 shadow-lg">
              <div className="flex items-center justify-between mb-6">
                <h2 className="text-lg font-bold text-white">Playtime History</h2>
                <div className="flex bg-gray-800 rounded-lg p-1" role="group" aria-label="Time range">
                  {(['7', '30'] as const).map((days) => (
                    <button
                      key={days}
                      onClick={() => setTimeRange(days)}
                      aria-pressed={timeRange === days}
                      className={`px-3 py-1 rounded text-xs font-medium transition-colors ${timeRange === days ? 'bg-indigo-600 text-white shadow-sm' : 'text-gray-400 hover:text-white'}`}
                    >
                      {days}D
                    </button>
                  ))}
                </div>
              </div>
              <div className="h-64">
                <PlaytimeChart data={history} bare />
              </div>
            </div>

            <DetectionCard gameId={game.id} />
          </div>

          <SessionHistory gameId={game.id} />
        </div>
      </div>
    </div>
  );
}

function Stat({ icon, label, value, detail }: { icon: React.ReactNode; label: string; value: string; detail?: string }) {
  return (
    <div className="bg-gray-900 border border-gray-800 rounded-xl p-5 shadow-lg">
      <div className="flex items-center text-gray-400 mb-2">
        <span className="mr-2 text-indigo-400">{icon}</span>
        <span className="text-sm font-medium uppercase tracking-wider">{label}</span>
      </div>
      <div className="text-3xl font-bold text-white">{value}</div>
      {detail && <div className="text-xs text-gray-500 mt-1">{detail}</div>}
    </div>
  );
}

/** The game's name, which turns into a text field to rename it. */
function GameTitle({ game, onRenamed }: { game: GameLibraryItem; onRenamed: (name: string) => void }) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(game.name);

  const save = async () => {
    const trimmed = name.trim();
    if (trimmed === game.name) {
      setEditing(false);
      return;
    }
    try {
      await invoke('rename_game', { gameId: game.id, name: trimmed });
      onRenamed(trimmed);
      setEditing(false);
    } catch (e) {
      reportError("Couldn't rename the game", e);
    }
  };

  if (editing) {
    return (
      <input
        autoFocus
        value={name}
        onChange={(e) => setName(e.target.value)}
        onBlur={save}
        onKeyDown={(e) => {
          if (e.key === 'Enter') save();
          if (e.key === 'Escape') {
            setName(game.name);
            setEditing(false);
          }
        }}
        aria-label="Game name"
        className="text-3xl font-bold text-white bg-gray-800 border border-indigo-500 rounded-lg px-3 py-1 focus:outline-none min-w-0"
      />
    );
  }

  return (
    <h1 className="group text-4xl font-bold text-white tracking-tight flex items-center gap-3 min-w-0">
      <span className="truncate">{game.name}</span>
      <button
        onClick={() => {
          setName(game.name);
          setEditing(true);
        }}
        className="text-gray-500 hover:text-white opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity"
        aria-label="Rename game"
      >
        <Pencil size={18} />
      </button>
    </h1>
  );
}

function ActionsMenu({ onChangeCover, onRemove }: { onChangeCover: () => void; onRemove: () => void }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && setOpen(false);
    document.addEventListener('mousedown', close);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', close);
      document.removeEventListener('keydown', onKey);
    };
  }, [open]);

  const item = 'w-full flex items-center gap-2 px-4 py-2 text-sm text-left';
  return (
    <div className="relative" ref={ref}>
      <button
        onClick={() => setOpen(!open)}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label="More actions"
        className="p-2 rounded-lg border bg-gray-800 border-gray-700 text-gray-200 hover:bg-gray-700"
      >
        <MoreHorizontal size={18} />
      </button>
      {open && (
        <div role="menu" className="absolute right-0 mt-2 w-52 bg-gray-800 border border-gray-700 rounded-xl shadow-2xl py-1 z-20">
          <button role="menuitem" className={`${item} text-gray-200 hover:bg-gray-700`} onClick={() => { setOpen(false); onChangeCover(); }}>
            <Image size={16} /> Change cover
          </button>
          <button role="menuitem" className={`${item} text-red-300 hover:bg-red-900/40`} onClick={() => { setOpen(false); onRemove(); }}>
            <Trash2 size={16} /> Remove from library
          </button>
        </div>
      )}
    </div>
  );
}
