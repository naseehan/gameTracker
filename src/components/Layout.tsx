import { ReactNode } from 'react';
import { Home, Library, BarChart2, Settings, Play } from 'lucide-react';
import clsx from 'clsx';
import { useNavigationStore, Route } from '../store/navigationStore';
import { useSessionsStore } from '../store/sessionsStore';
import { useBackendSync } from '../hooks/useBackendEvent';
import { LiveTimer } from './LiveTimer';
import { GameCover } from './GameCover';

interface LayoutProps {
  children: ReactNode;
}

const navItems: { id: Route['type']; label: string; icon: typeof Home }[] = [
  { id: 'home', label: 'Home', icon: Home },
  { id: 'library', label: 'Library', icon: Library },
  { id: 'stats', label: 'Statistics', icon: BarChart2 },
  { id: 'settings', label: 'Settings', icon: Settings },
];

export function Layout({ children }: LayoutProps) {
  useBackendSync();
  const { currentRoute, navigate } = useNavigationStore();
  const activeTab = currentRoute.type === 'game_details' ? 'library' : currentRoute.type;

  return (
    <div className="flex h-screen overflow-hidden bg-gray-950 text-gray-100 font-sans selection:bg-indigo-500/30">
      {/* Sidebar */}
      <aside className="w-60 bg-gray-900 border-r border-gray-800 flex flex-col p-4 z-10 shadow-xl flex-shrink-0">
        <div className="flex items-center space-x-3 mb-8 px-2">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center shadow-lg shadow-indigo-500/20">
            <Play size={16} fill="currentColor" className="text-white ml-0.5" />
          </div>
          <h1 className="text-lg font-bold tracking-tight text-gray-100">GameTracker</h1>
        </div>

        <nav className="space-y-1 flex-1" aria-label="Main">
          {navItems.map((item) => (
            <button
              key={item.id}
              onClick={() => navigate({ type: item.id } as Route)}
              aria-current={activeTab === item.id ? 'page' : undefined}
              className={clsx(
                'w-full flex items-center space-x-3 px-3 py-2.5 rounded-lg transition-colors text-sm font-medium',
                activeTab === item.id
                  ? 'bg-gray-800 text-white shadow-sm'
                  : 'text-gray-400 hover:bg-gray-800/50 hover:text-gray-200',
              )}
            >
              <item.icon size={18} className={activeTab === item.id ? 'text-indigo-400' : 'text-gray-500'} />
              <span>{item.label}</span>
            </button>
          ))}
        </nav>

        <NowPlaying />
      </aside>

      {/* Main Content Area */}
      <main className="flex-1 flex flex-col overflow-hidden relative min-w-0">
        <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_top_right,_var(--tw-gradient-stops))] from-indigo-900/10 via-gray-950 to-gray-950 -z-10"></div>
        {children}
      </main>
    </div>
  );
}

/** The games being played right now, with a running timer. */
function NowPlaying() {
  const active = useSessionsStore((s) => s.active);
  const navigate = useNavigationStore((s) => s.navigate);

  if (active.length === 0) {
    return (
      <div className="px-3 py-3 rounded-lg border border-gray-800 text-xs text-gray-500">
        Nothing playing. Launch a game and it's tracked automatically.
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {active.map((session) => (
        <button
          key={session.session_id}
          onClick={() => navigate({ type: 'game_details', gameId: session.game_id })}
          className="w-full flex items-center gap-3 p-2 rounded-lg bg-green-500/10 border border-green-500/30 hover:bg-green-500/15 text-left transition-colors"
        >
          <div className="w-9 h-12 rounded overflow-hidden flex-shrink-0">
            <GameCover path={session.cover_path} alt="" iconSize={16} className="w-full h-full" />
          </div>
          <div className="min-w-0">
            <p className="text-[11px] font-semibold uppercase tracking-wider text-green-400 flex items-center gap-1.5">
              <span className="w-1.5 h-1.5 rounded-full bg-green-400 animate-pulse"></span>
              Playing
            </p>
            <p className="text-sm font-medium text-white truncate">{session.game_name}</p>
            <LiveTimer startedAt={session.started_at} className="text-xs text-gray-400" />
          </div>
        </button>
      ))}
    </div>
  );
}
