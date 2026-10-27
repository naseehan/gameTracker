import { useEffect, useState } from 'react';
import { Search, Plus, Filter, SortDesc } from 'lucide-react';
import { useLibraryStore, SortOption } from '../store/libraryStore';
import { GameCard } from '../components/GameCard';
import { AddGameModal } from '../components/AddGameModal';
import { ScanModal } from '../components/ScanModal';
import { DiscoveryModal } from '../components/DiscoveryModal';
import { useNavigationStore } from '../store/navigationStore';
import { Dropdown } from '../components/Dropdown';

export function Library() {
  const { navigate } = useNavigationStore();
  const { games, isLoading, searchQuery, sortBy, filterBy, setSearchQuery, setSortBy, setFilterBy, fetchGames, filteredAndSortedGames, getAvailablePlatforms } = useLibraryStore();
  
  const [showAddModal, setShowAddModal] = useState(false);
  const [showScanModal, setShowScanModal] = useState(false);
  const [showDiscoveryModal, setShowDiscoveryModal] = useState(false);

  useEffect(() => {
    fetchGames();
    const interval = setInterval(fetchGames, 5000);
    return () => clearInterval(interval);
  }, []);

  const displayedGames = filteredAndSortedGames();
  const platforms = getAvailablePlatforms();

  if (isLoading && games.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="animate-spin w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full"></div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden">
      {/* Top Bar */}
      <div className="flex flex-wrap items-center justify-between p-6 border-b border-gray-800 bg-gray-900/50 flex-shrink-0 gap-4">
        <div className="flex-1 min-w-[200px] max-w-md relative">
          <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
            <Search size={18} className="text-gray-500" />
          </div>
          <input
            type="text"
            className="block w-full pl-10 pr-3 py-2 border border-gray-700 rounded-lg leading-5 bg-gray-900 text-gray-300 placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
            placeholder="Search library..."
            aria-label="Search library"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>
        
        <div className="flex items-center space-x-3">
          <Dropdown 
            value={filterBy} 
            onChange={(v) => setFilterBy(v)}
            icon={<Filter size={16} className="text-gray-500" />}
            options={[
              { value: 'all', label: 'All Games' },
              { value: 'installed', label: 'Installed' },
              { value: 'not_installed', label: 'Not Installed' },
              { value: 'currently_playing', label: 'Currently Playing' },
              { value: 'ignored', label: 'Ignored' },
              ...(platforms.length > 0 ? [{ value: 'divider', label: '', disabled: true }] : []),
              ...platforms.map(p => ({ value: p, label: p }))
            ]}
          />

          <Dropdown 
            value={sortBy} 
            onChange={(v) => setSortBy(v as SortOption)}
            icon={<SortDesc size={16} className="text-gray-500" />}
            options={[
              { value: 'recently_played', label: 'Recently Played' },
              { value: 'most_played', label: 'Most Played' },
              { value: 'recently_added', label: 'Recently Added' },
              { value: 'alphabetical', label: 'Alphabetical' }
            ]}
          />

          <button
            onClick={() => setShowDiscoveryModal(true)}
            className="flex items-center px-4 py-2 bg-gray-800 hover:bg-gray-700 text-white border border-gray-700 rounded-lg transition-colors font-medium text-sm"
          >
            <Search size={18} className="mr-1" />
            Auto Discover
          </button>
          
          <button
            onClick={() => setShowAddModal(true)}
            className="flex items-center px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg transition-colors font-medium text-sm"
          >
            <Plus size={18} className="mr-1" />
            Add Game
          </button>
        </div>
      </div>

      {/* Grid */}
      <div className="flex-1 overflow-y-auto p-6 bg-gray-950">
        {displayedGames.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-gray-500">
            <Search size={48} className="mb-4 opacity-20" />
            <p className="text-lg font-medium text-gray-400">No games found</p>
            {searchQuery || filterBy !== 'all' ? (
              <p className="text-sm mt-2">Try adjusting your search or filters.</p>
            ) : (
              <p className="text-sm mt-2">Add some games to get started.</p>
            )}
          </div>
        ) : (
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-6">
            {displayedGames.map(game => (
              <div key={game.id} onClick={() => navigate({ type: 'game_details', gameId: game.id })} className="cursor-pointer">
                <GameCard 
                  game={game} 
                  onClick={() => {}}
                />
              </div>
            ))}
          </div>
        )}
      </div>

      {showAddModal && (
        <AddGameModal 
          onClose={() => setShowAddModal(false)}
        />
      )}
      
      {showScanModal && (
        <ScanModal
          onClose={() => setShowScanModal(false)}
        />
      )}
      
      {showDiscoveryModal && (
        <DiscoveryModal
          onClose={() => setShowDiscoveryModal(false)}
        />
      )}
    </div>
  );
}