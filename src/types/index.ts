export interface GameLibraryItem {
  id: number;
  name: string;
  cover_path: string | null;
  total_playtime_seconds: number;
  last_played_at: string | null;
  currently_playing: boolean;
  created_at: string | null;
  is_ignored: boolean;
  is_removed: boolean;
  current_install_status: string;
  platforms: string[];
  installation_paths: string[];
}

/** A running session, with its game's name and cover. */
export interface ActiveSessionInfo {
  session_id: number;
  game_id: number;
  game_name: string;
  cover_path: string | null;
  started_at: string;
}

/** A finished session, with its game's name and cover. */
export interface RecentSession {
  id: number;
  game_id: number;
  game_name: string;
  cover_path: string | null;
  started_at: string;
  ended_at: string;
  duration_seconds: number;
}

export interface GameCandidate {
  name: string;
  install_path: string;
  executables: string[];
  source: string;
  confidence: number;
  platform_app_id?: string | null;
}

export interface GameStatistics {
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

export interface GameSessionDto {
  session_id: number;
  started_at: string;
  ended_at: string | null;
  duration_seconds: number;
  is_active: boolean;
}

export interface DailyPlaytime {
  date_label: string;
  playtime_seconds: number;
}

export interface DetectionEntry {
  id: number;
  path: string;
}

export interface GameDetection {
  executables: DetectionEntry[];
  install_folders: DetectionEntry[];
}

export interface RemovedGame {
  id: number;
  name: string;
  total_playtime_seconds: number;
}

export interface UpdateStatus {
  configured: boolean;
  current_version: string;
  available_version: string | null;
  notes: string | null;
}
