import { useEffect } from 'react';
import { useStatisticsStore, TimeRange } from '../store/statisticsStore';
import { StatCard } from '../components/statistics/StatCard';
import { PlaytimeChart } from '../components/statistics/PlaytimeChart';
import { TopGamesChart } from '../components/statistics/TopGamesChart';
import { ActivityChart } from '../components/statistics/ActivityChart';
import { formatPlaytime } from '../utils/formatters';
import { Clock, Gamepad2, Trophy, Target, BarChart2 } from 'lucide-react';
import clsx from 'clsx';

export function Statistics() {
  const { stats, isLoading, error, timeRange, setTimeRange, fetchStats } = useStatisticsStore();

  useEffect(() => {
    fetchStats();
  }, []);

  if (isLoading && !stats) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="animate-spin w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex-1 flex items-center justify-center p-8">
        <div className="bg-red-500/10 border border-red-500 text-red-400 p-6 rounded-lg max-w-md text-center">
          <h3 className="font-bold mb-2">Failed to load statistics</h3>
          <p className="text-sm">{error}</p>
          <button onClick={() => fetchStats()} className="mt-4 px-4 py-2 bg-red-500 hover:bg-red-600 text-white rounded transition-colors">Retry</button>
        </div>
      </div>
    );
  }

  if (!stats || (stats.total_playtime_seconds === 0 && stats.games_played === 0)) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center p-8 text-center max-w-md mx-auto">
        <div className="w-24 h-24 bg-gray-900 rounded-full flex items-center justify-center mb-6 shadow-xl border border-gray-800">
          <BarChart2 size={40} className="text-gray-700" />
        </div>
        <h3 className="text-xl font-bold text-gray-200 mb-2">No gaming data yet</h3>
        <p className="text-gray-500 mb-8">Start playing a tracked game and your statistics will appear here.</p>
      </div>
    );
  }

  const timeRanges: { id: TimeRange; label: string }[] = [
    { id: 'today', label: 'Today' },
    { id: '7d', label: 'Last 7 Days' },
    { id: '30d', label: 'Last 30 Days' },
    { id: 'all', label: 'All Time' }
  ];

  return (
    <div className="flex flex-col h-full overflow-y-auto">
      <header className="px-8 py-6 border-b border-gray-800/50 backdrop-blur-md sticky top-0 z-10 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold text-white tracking-wide">Dashboard & Statistics</h2>
        </div>
        <div className="flex bg-gray-900 border border-gray-700 rounded-lg p-1">
          {timeRanges.map(range => (
            <button
              key={range.id}
              onClick={() => setTimeRange(range.id)}
              className={clsx(
                "px-4 py-1.5 text-sm font-medium rounded-md transition-colors",
                timeRange === range.id 
                  ? "bg-indigo-600 text-white shadow-sm" 
                  : "text-gray-400 hover:text-gray-200 hover:bg-gray-800/50"
              )}
            >
              {range.label}
            </button>
          ))}
        </div>
      </header>

      <div className="p-8 space-y-6">
        {/* Summary Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <StatCard 
            title="Total Playtime" 
            value={formatPlaytime(stats.total_playtime_seconds)} 
            icon={<Clock size={20} />} 
          />
          <StatCard 
            title="Games Played" 
            value={stats.games_played} 
            subtitle={`${stats.total_sessions} total sessions`}
            icon={<Gamepad2 size={20} />} 
          />
          <StatCard 
            title="Most Played Game" 
            value={stats.most_played_game || 'None'} 
            subtitle={stats.most_played_game ? formatPlaytime(stats.most_played_game_playtime) : undefined}
            icon={<Trophy size={20} />} 
          />
          <StatCard 
            title="This Week" 
            value={formatPlaytime(stats.week_playtime_seconds)}
            subtitle={`Today: ${formatPlaytime(stats.today_playtime_seconds)}`}
            icon={<Target size={20} />} 
          />
        </div>

        {/* Main Chart */}
        <PlaytimeChart data={stats.playtime_over_time} />

        {/* Secondary Charts */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <TopGamesChart data={stats.top_games} />
          <ActivityChart data={stats.activity_by_day_of_week} />
        </div>
      </div>
    </div>
  );
}


