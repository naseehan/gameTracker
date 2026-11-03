import { GameLibraryItem } from '../types';
import { GameCover } from './GameCover';
import { formatPlaytime } from '../utils/formatters';

interface GameCardProps {
  game: GameLibraryItem;
  onClick: (game: GameLibraryItem) => void;
}

export function GameCard({ game, onClick }: GameCardProps) {
  const isInstalled = game.current_install_status !== 'Not Installed';

  return (
    <button
      type="button"
      onClick={() => onClick(game)}
      title={game.name}
      className={`group relative w-full text-left rounded-xl overflow-hidden cursor-pointer transition-all duration-200 hover:-translate-y-1 focus-visible:-translate-y-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500 hover:shadow-2xl hover:shadow-indigo-500/20 bg-gray-900 border border-gray-800 ${game.is_ignored ? 'opacity-50 grayscale hover:grayscale-0' : ''}`}
    >
      <div className="aspect-[3/4] relative">
        <GameCover
          path={game.cover_path}
          alt=""
          iconSize={56}
          className={`w-full h-full group-hover:text-indigo-400 transition-colors ${!isInstalled ? 'grayscale opacity-70' : ''}`}
        />
        <div className="absolute inset-0 bg-gradient-to-t from-black/90 via-black/30 to-transparent"></div>

        <div className="absolute top-2 left-2 right-2 flex flex-wrap gap-1.5">
          {game.currently_playing && (
            <span className="bg-green-500 text-white text-[11px] font-bold px-2 py-0.5 rounded shadow-lg shadow-green-500/30">
              PLAYING
            </span>
          )}
          {!isInstalled && (
            <span className="bg-gray-950/80 text-gray-300 text-[11px] font-semibold px-2 py-0.5 rounded">
              Not installed
            </span>
          )}
          {game.is_ignored && (
            <span className="bg-gray-950/80 text-gray-300 text-[11px] font-semibold px-2 py-0.5 rounded">
              Not tracked
            </span>
          )}
        </div>

        <div className="absolute bottom-0 left-0 right-0 p-3">
          <h3 className="font-bold text-white text-sm leading-snug line-clamp-2 drop-shadow-md">{game.name}</h3>
          <p className="text-indigo-300 text-xs font-medium mt-1 drop-shadow">
            {game.total_playtime_seconds > 0 ? formatPlaytime(game.total_playtime_seconds) : 'Not played yet'}
          </p>
        </div>
      </div>
    </button>
  );
}
