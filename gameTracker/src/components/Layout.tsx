import { ReactNode, useState } from 'react';
import { Library, Clock, Play, BarChart2, Settings, Plus } from 'lucide-react';
import clsx from 'clsx';
import { AddGameModal } from './AddGameModal';
import { useNavigationStore } from '../store/navigationStore';

interface LayoutProps {
  children: ReactNode;
}

export function Layout({ children }: LayoutProps) {
  const [showAddModal, setShowAddModal] = useState(false);
  const { currentRoute, navigate } = useNavigationStore();
  
  const activeTab = currentRoute.type === 'game_details' ? 'library' : currentRoute.type;

  const navItems = [
    { id: 'library', label: 'Library', icon: Library },
    { id: 'recent', label: 'Recently Played', icon: Clock },
    { id: 'playing', label: 'Currently Playing', icon: Play },
  ];

  const bottomNavItems = [
    { id: 'stats', label: 'Statistics', icon: BarChart2 },
    { id: 'settings', label: 'Settings', icon: Settings },
  ];

  return (
    <div className="flex h-screen overflow-hidden bg-gray-950 text-gray-100 font-sans selection:bg-primary/30">
      {/* Sidebar */}
      <aside className="w-64 bg-gray-900 border-r border-gray-800 flex flex-col p-4 z-10 shadow-xl">
        <div className="flex items-center space-x-3 mb-8 px-2">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center shadow-lg shadow-indigo-500/20">
            <Play size={16} fill="currentColor" className="text-white ml-0.5" />
          </div>
          <h1 className="text-xl font-bold tracking-widest text-transparent bg-clip-text bg-gradient-to-r from-gray-100 to-gray-400">
            TRACKER
          </h1>
        </div>

        <nav className="space-y-1 flex-1">
          <div className="text-xs font-semibold text-gray-500 tracking-wider mb-3 px-2 mt-4">COLLECTION</div>
          {navItems.map(item => (
            <button
              key={item.id}
              onClick={() => navigate({ type: item.id as any })}
              className={clsx(
                "w-full flex items-center space-x-3 px-3 py-2.5 rounded-lg transition-all text-sm font-medium",
                activeTab === item.id 
                  ? "bg-gray-800 text-white shadow-sm" 
                  : "text-gray-400 hover:bg-gray-800/50 hover:text-gray-200"
              )}
            >
              <item.icon size={18} className={activeTab === item.id ? "text-indigo-400" : "text-gray-500"} />
              <span>{item.label}</span>
            </button>
          ))}
          
          <div className="text-xs font-semibold text-gray-500 tracking-wider mb-3 px-2 mt-8">SYSTEM</div>
          {bottomNavItems.map(item => (
            <button
              key={item.id}
              onClick={() => navigate({ type: item.id as any })}
              className={clsx(
                "w-full flex items-center space-x-3 px-3 py-2.5 rounded-lg transition-all text-sm font-medium",
                activeTab === item.id 
                  ? "bg-gray-800 text-white shadow-sm" 
                  : "text-gray-400 hover:bg-gray-800/50 hover:text-gray-200"
              )}
            >
              <item.icon size={18} className={activeTab === item.id ? "text-indigo-400" : "text-gray-500"} />
              <span>{item.label}</span>
            </button>
          ))}
        </nav>

        {/* Add Game Button */}
        <button 
          onClick={() => setShowAddModal(true)}
          className="mt-auto flex items-center justify-center space-x-2 w-full py-3 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg transition-colors font-medium text-sm shadow-lg shadow-indigo-600/20"
        >
          <Plus size={18} />
          <span>Add Game</span>
        </button>
      </aside>

      {/* Main Content Area */}
      <main className="flex-1 flex flex-col overflow-hidden relative">
        <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_top_right,_var(--tw-gradient-stops))] from-indigo-900/10 via-gray-950 to-gray-950 -z-10"></div>
        {children}
      </main>

      {showAddModal && <AddGameModal onClose={() => setShowAddModal(false)} />}
    </div>
  );
}