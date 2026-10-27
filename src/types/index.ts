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

export interface GameSession {
  id: number;
  game_id: number;
  started_at: string;
  ended_at: string | null;
  duration_seconds: number | null;
  detection_source: string | null;
}

export interface GameCandidate {
  name: string;
  install_path: string;
  executables: string[];
  source: string;
  confidence: number;
}
