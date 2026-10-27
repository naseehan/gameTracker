import { Gamepad2 } from 'lucide-react';
import { GameLibraryItem } from '../types';
import { convertFileSrc } from '@tauri-apps/api/core';

interface GameCardProps {
  game: GameLibraryItem;
  onClick: (game: GameLibraryItem) => void;
}

export function GameCard({ game, onClick }: GameCardProps) {
  // @ts-ignore - current_install_status might not be strongly typed on frontend yet but comes from backend
  const isInstalled = game.current_install_status !== 'Not Installed';

  return (
    <div 
      onClick={() => onClick(game)}
      onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onClick(game); } }}
      tabIndex={0}
      role="button"
      className={`group relative rounded-xl overflow-hidden cursor-pointer transition-all duration-300 hover:scale-105 focus:scale-105 focus:outline-none focus:ring-2 focus:ring-indigo-500 hover:shadow-2xl hover:shadow-indigo-500/20 bg-gray-900 border border-gray-800 ${game.is_ignored ? 'opacity-50 grayscale hover:grayscale-0' : ''}`}
    >
      <div className="aspect-[3/4] relative">
        {game.cover_path ? (
          <img 
            src={convertFileSrc(game.cover_path)}
            alt={game.name}
            className={`w-full h-full object-cover ${!isInstalled ? 'grayscale opacity-70' : ''}`}
            onError={(e) => {
              (e.target as HTMLImageElement).src = ''; // Fallback
            }}
          />
        ) : (
          <div className="w-full h-full flex items-center justify-center bg-gray-800 text-gray-600 group-hover:text-indigo-400 transition-colors">
            <Gamepad2 size={64} />
          </div>
        )}
        <div className="absolute inset-0 bg-gradient-to-t from-black/90 via-black/20 to-transparent"></div>
        
        {game.currently_playing && (
          <div className="absolute top-3 right-3 bg-green-500 text-white text-xs font-bold px-2 py-1 rounded shadow-lg shadow-green-500/30 animate-pulse">
            PLAYING
          </div>
        )}
        
        {!isInstalled && (
          <div className="absolute top-3 left-3 bg-red-500/90 text-white text-xs font-bold px-2 py-1 rounded shadow-lg">
            NOT INSTALLED
          </div>
        )}

        <div className="absolute bottom-0 left-0 right-0 p-4">
          <h3 className="font-bold text-white text-lg leading-tight truncate drop-shadow-md">{game.name}</h3>
          <div className="flex justify-between items-center mt-2">
            <p className="text-indigo-300 text-sm font-medium drop-shadow">
              {Math.floor(game.total_playtime_seconds / 3600)}h {Math.floor((game.total_playtime_seconds % 3600)/60)}m
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}