import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { GameLibraryItem } from '../types';

export type SortOption = 'recently_played' | 'most_played' | 'alphabetical' | 'recently_added';
export type FilterOption = 'all' | 'installed' | 'not_installed' | 'currently_playing' | 'ignored';

interface LibraryState {
  games: GameLibraryItem[];
  isLoading: boolean;
  error: string | null;
  searchQuery: string;
  sortBy: SortOption;
  filterBy: FilterOption | string;
  setSearchQuery: (query: string) => void;
  setSortBy: (sort: SortOption) => void;
  setFilterBy: (filter: FilterOption | string) => void;
  fetchGames: () => Promise<void>;
  filteredAndSortedGames: () => GameLibraryItem[];
  getAvailablePlatforms: () => string[];
}

export const useLibraryStore = create<LibraryState>((set, get) => ({
  games: [],
  isLoading: true,
  error: null,
  searchQuery: '',
  sortBy: 'recently_played',
  filterBy: 'all',

  setSearchQuery: (query) => set({ searchQuery: query }),
  setSortBy: (sort) => set({ sortBy: sort }),
  setFilterBy: (filter) => set({ filterBy: filter }),

  fetchGames: async () => {
    try {
      set({ error: null });
      if (get().games.length === 0) set({ isLoading: true });
      
      const games = await invoke<GameLibraryItem[]>('get_library');
      set({ games, isLoading: false });
    } catch (err: any) {
      set({ error: err.toString(), isLoading: false });
    }
  },
  
  getAvailablePlatforms: () => {
    const { games } = get();
    const platforms = new Set<string>();
    games.forEach(g => {
      if (g.platforms) {
        g.platforms.forEach(p => platforms.add(p));
      }
    });
    return Array.from(platforms).sort();
  },

  filteredAndSortedGames: () => {
    const { games, searchQuery, sortBy, filterBy } = get();
    
    // Filter
    let result = games;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(g => g.name.toLowerCase().includes(q));
    }
    
    if (filterBy !== 'all') {
      if (filterBy === 'installed') {
        result = result.filter(g => g.current_install_status === 'Installed');
      } else if (filterBy === 'not_installed') {
        result = result.filter(g => g.current_install_status !== 'Installed');
      } else if (filterBy === 'currently_playing') {
        result = result.filter(g => g.currently_playing);
      } else if (filterBy === 'ignored') {
        result = result.filter(g => g.is_ignored);
      } else {
        // Platform filter
        result = result.filter(g => g.platforms && g.platforms.includes(filterBy));
      }
    }
    
    // Sort
    result = [...result].sort((a, b) => {
      switch (sortBy) {
        case 'recently_played': {
          if (a.currently_playing && !b.currently_playing) return -1;
          if (!a.currently_playing && b.currently_playing) return 1;
          const aTime = a.last_played_at ? new Date(a.last_played_at).getTime() : 0;
          const bTime = b.last_played_at ? new Date(b.last_played_at).getTime() : 0;
          return bTime - aTime;
        }
        case 'most_played':
          return b.total_playtime_seconds - a.total_playtime_seconds;
        case 'alphabetical':
          return a.name.localeCompare(b.name);
        case 'recently_added': {
          const aCreated = a.created_at ? new Date(a.created_at).getTime() : 0;
          const bCreated = b.created_at ? new Date(b.created_at).getTime() : 0;
          return bCreated - aCreated;
        }
        default:
          return 0;
      }
    });
    
    return result;
  }
}));