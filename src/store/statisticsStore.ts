import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { DailyPlaytime } from '../types';

export interface TopGame {
  game_id: number;
  name: string;
  playtime_seconds: number;
}

export interface DayOfWeekActivity {
  day: string;
  playtime_seconds: number;
}

export interface DashboardStats {
  total_playtime_seconds: number;
  games_played: number;
  total_sessions: number;
  today_playtime_seconds: number;
  week_playtime_seconds: number;
  most_played_game: string | null;
  most_played_game_playtime: number;
  playtime_over_time: DailyPlaytime[];
  top_games: TopGame[];
  activity_by_day_of_week: DayOfWeekActivity[];
}

export type TimeRange = 'today' | '7d' | '30d' | 'all';

interface StatisticsState {
  stats: DashboardStats | null;
  isLoading: boolean;
  error: string | null;
  timeRange: TimeRange;
  setTimeRange: (range: TimeRange) => void;
  fetchStats: () => Promise<void>;
}

export const useStatisticsStore = create<StatisticsState>((set, get) => ({
  stats: null,
  isLoading: true,
  error: null,
  timeRange: '7d',
  
  setTimeRange: (range: TimeRange) => {
    set({ timeRange: range });
    get().fetchStats();
  },

  fetchStats: async () => {
    const { timeRange } = get();
    try {
      set({ error: null, isLoading: true });
      const stats = await invoke<DashboardStats>('get_dashboard_stats', { timeRange });
      set({ stats, isLoading: false });
    } catch (err: any) {
      set({ error: err.toString(), isLoading: false });
    }
  }
}));
