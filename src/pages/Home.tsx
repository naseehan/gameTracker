import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Gamepad2 } from 'lucide-react';
import { RecentSession } from '../types';
import { DashboardStats } from '../store/statisticsStore';
import { useSessionsStore } from '../store/sessionsStore';
import { useNavigationStore } from '../store/navigationStore';
import { useBackendEvent } from '../hooks/useBackendEvent';
import { GameCover } from '../components/GameCover';
import { LiveTimer } from '../components/LiveTimer';
import { PlaytimeChart } from '../components/statistics/PlaytimeChart';
import { StatCard } from '../components/statistics/StatCard';
import { formatDayHeading, formatPlaytime, formatTime } from '../utils/formatters';
import { reportError } from '../utils/errors';

interface DayGroup {
  key: string;
  date: Date;
  total: number;
  sessions: RecentSession[];
}

function groupByDay(sessions: RecentSession[]): DayGroup[] {
  const groups = new Map<string, DayGroup>();
  for (const session of sessions) {
    const start = new Date(session.started_at);
    const key = start.toDateString();
    const group = groups.get(key) ?? { key, date: start, total: 0, sessions: [] };
    group.total += session.duration_seconds;
    group.sessions.push(session);
    groups.set(key, group);
  }
  return [...groups.values()];
}

export function Home() {
  const active = useSessionsStore((s) => s.active);
  const navigate = useNavigationStore((s) => s.navigate);
  const [week, setWeek] = useState<DashboardStats | null>(null);
  const [recent, setRecent] = useState<RecentSession[] | null>(null);

  const load = () => {
    invoke<DashboardStats>('get_dashboard_stats', { timeRange: '7d' })
      .then(setWeek)
      .catch((e) => reportError("Couldn't load this week's playtime", e));
    invoke<RecentSession[]>('get_recent_sessions', { limit: 40 })
      .then(setRecent)
      .catch((e) => reportError("Couldn't load recent sessions", e));
  };

  useEffect(load, []);
  useBackendEvent(['sessions-changed', 'library-changed'], load);

  // Running totals include the current session; refresh them every minute while playing.
  useEffect(() => {
    if (active.length === 0) return;
    const interval = setInterval(load, 60_000);
    return () => clearInterval(interval);
  }, [active.length]);

  const openGame = (gameId: number) => navigate({ type: 'game_details', gameId });
  const days = groupByDay(recent ?? []);
  const nothingYet = recent !== null && recent.length === 0 && active.length === 0;

  return (
    <div className="flex-1 overflow-y-auto">
      <div className="max-w-5xl mx-auto p-8 space-y-10">
        <header>
          <h1 className="text-3xl font-bold text-white tracking-tight">Home</h1>
        </header>

        {active.length > 0 && (
          <section aria-labelledby="now-playing" className="space-y-4">
            <h2 id="now-playing" className="text-sm font-semibold uppercase tracking-wider text-gray-500">Now playing</h2>
            {active.map((session) => (
              <button
                key={session.session_id}
                onClick={() => openGame(session.game_id)}
                className="w-full flex items-center gap-6 p-5 bg-gray-900 border border-green-500/30 rounded-2xl hover:border-green-500/60 transition-colors text-left"
              >
                <div className="w-20 h-28 rounded-lg overflow-hidden flex-shrink-0">
                  <GameCover path={session.cover_path} alt="" iconSize={28} className="w-full h-full" />
                </div>
                <div className="min-w-0 flex-1">
                  <p className="text-xs font-semibold uppercase tracking-wider text-green-400 flex items-center gap-2 mb-1">
                    <span className="w-2 h-2 rounded-full bg-green-400 animate-pulse"></span>
                    Playing since {formatTime(session.started_at)}
                  </p>
                  <p className="text-2xl font-bold text-white truncate">{session.game_name}</p>
                </div>
                <LiveTimer startedAt={session.started_at} className="text-3xl font-bold text-white" />
              </button>
            ))}
          </section>
        )}

        {nothingYet ? (
          <div className="flex flex-col items-center text-center py-16">
            <div className="w-20 h-20 bg-gray-900 rounded-full flex items-center justify-center mb-6 border border-gray-800">
              <Gamepad2 size={36} className="text-gray-600" />
            </div>
            <h2 className="text-xl font-bold text-white mb-2">No play sessions yet</h2>
            <p className="text-gray-400 max-w-sm mb-6">
              Launch any game in your library and GameTracker records it automatically, even with this window closed.
            </p>
            <button
              onClick={() => navigate({ type: 'library' })}
              className="px-6 py-3 bg-indigo-600 hover:bg-indigo-500 text-white rounded-xl font-medium"
            >
              Open library
            </button>
          </div>
        ) : (
          <>
            <section aria-labelledby="this-week" className="space-y-4">
              <h2 id="this-week" className="text-sm font-semibold uppercase tracking-wider text-gray-500">Last 7 days</h2>
              <div className="grid grid-cols-3 gap-4">
                <StatCard title="Played" value={week ? formatPlaytime(week.total_playtime_seconds) : '–'} />
                <StatCard title="Today" value={week ? formatPlaytime(week.today_playtime_seconds) : '–'} />
                <StatCard
                  title="Most played"
                  value={week?.most_played_game ?? '–'}
                  subtitle={week?.most_played_game ? formatPlaytime(week.most_played_game_playtime) : undefined}
                />
              </div>
              {week && <PlaytimeChart data={week.playtime_over_time} title="Playtime per day" />}
            </section>

            <section aria-labelledby="recent" className="space-y-6">
              <h2 id="recent" className="text-sm font-semibold uppercase tracking-wider text-gray-500">Recent sessions</h2>
              {days.map((day) => (
                <div key={day.key}>
                  <div className="flex items-baseline justify-between mb-2 px-1">
                    <h3 className="font-semibold text-gray-200">{formatDayHeading(day.date)}</h3>
                    <span className="text-sm text-gray-500">{formatPlaytime(day.total)}</span>
                  </div>
                  <ul className="bg-gray-900 border border-gray-800 rounded-xl divide-y divide-gray-800">
                    {day.sessions.map((session) => (
                      <li key={session.id}>
                        <button
                          onClick={() => openGame(session.game_id)}
                          className="w-full flex items-center gap-4 px-4 py-3 hover:bg-gray-800/50 text-left first:rounded-t-xl last:rounded-b-xl"
                        >
                          <div className="w-8 h-11 rounded overflow-hidden flex-shrink-0">
                            <GameCover path={session.cover_path} alt="" iconSize={14} className="w-full h-full" />
                          </div>
                          <span className="flex-1 min-w-0 font-medium text-gray-200 truncate">{session.game_name}</span>
                          <span className="text-sm text-gray-500 tabular-nums">
                            {formatTime(session.started_at)} – {formatTime(session.ended_at)}
                          </span>
                          <span className="w-20 text-right text-sm font-medium text-gray-300 tabular-nums">
                            {formatPlaytime(session.duration_seconds)}
                          </span>
                        </button>
                      </li>
                    ))}
                  </ul>
                </div>
              ))}
            </section>
          </>
        )}
      </div>
    </div>
  );
}
