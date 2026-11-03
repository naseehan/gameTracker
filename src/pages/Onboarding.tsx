import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Check, FolderSearch, Loader2, Play } from 'lucide-react';
import { GameCandidate } from '../types';
import { ScanModal } from '../components/ScanModal';
import { groupBySource } from '../components/DiscoveryModal';
import { Toggle } from '../components/Toggle';
import { describeError, reportError } from '../utils/errors';

interface OnboardingProps {
  onComplete: () => void;
}

type Step = 'welcome' | 'finding' | 'found' | 'done';
const STEPS: Step[] = ['welcome', 'finding', 'found', 'done'];

function listNames(names: string[]): string {
  if (names.length <= 1) return names.join('');
  return `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`;
}

export function Onboarding({ onComplete }: OnboardingProps) {
  const [step, setStep] = useState<Step>('welcome');
  const [candidates, setCandidates] = useState<GameCandidate[]>([]);
  const [discoveryError, setDiscoveryError] = useState<string | null>(null);
  const [showScan, setShowScan] = useState(false);
  const [autostart, setAutostart] = useState(false);

  const findGames = async () => {
    setStep('finding');
    setDiscoveryError(null);
    try {
      setCandidates(await invoke<GameCandidate[]>('trigger_discovery'));
    } catch (err) {
      setDiscoveryError(describeError(err));
    }
    setStep('found');
  };

  const changeAutostart = async (enabled: boolean) => {
    try {
      await invoke('set_autostart_enabled', { enabled });
      setAutostart(enabled);
    } catch (err) {
      reportError("Couldn't change the startup setting", err);
    }
  };

  const finish = async () => {
    try {
      await invoke('complete_onboarding');
      onComplete();
    } catch (err) {
      reportError("Couldn't finish setup", err);
    }
  };

  const grouped = groupBySource(candidates);
  const launchers = Object.keys(grouped);

  return (
    <div className="flex items-center justify-center min-h-screen bg-gray-950 text-gray-100 p-8 w-full z-40 fixed inset-0">
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,_var(--tw-gradient-stops))] from-indigo-900/20 via-gray-950 to-gray-950 -z-10"></div>

      <div className="w-full max-w-2xl bg-gray-900/80 backdrop-blur-md border border-gray-800 p-10 rounded-2xl shadow-2xl relative overflow-hidden">
        <div className="absolute top-0 left-0 h-1 bg-gray-800 w-full" aria-hidden="true">
          <div
            className="h-full bg-indigo-500 transition-all duration-500 ease-out"
            style={{ width: `${((STEPS.indexOf(step) + 1) / STEPS.length) * 100}%` }}
          />
        </div>

        {step === 'welcome' && (
          <div className="flex flex-col items-center text-center">
            <div className="w-16 h-16 bg-gradient-to-br from-indigo-500 to-purple-600 rounded-2xl flex items-center justify-center mb-8 shadow-lg shadow-indigo-500/20">
              <Play size={30} fill="currentColor" className="text-white ml-1" />
            </div>
            <h1 className="text-4xl font-bold mb-4">Welcome to GameTracker</h1>
            <p className="text-lg text-gray-400 mb-10 max-w-md">
              It records how long you play each game, automatically, and keeps everything on this PC.
            </p>
            <button
              onClick={findGames}
              className="px-8 py-4 bg-indigo-600 hover:bg-indigo-500 text-white font-medium rounded-xl transition-colors shadow-lg shadow-indigo-600/20"
            >
              Find my games
            </button>
          </div>
        )}

        {step === 'finding' && (
          <div className="flex flex-col items-center text-center py-10">
            <Loader2 className="w-12 h-12 text-indigo-500 animate-spin mb-6" />
            <h2 className="text-2xl font-bold mb-2">Looking for your games</h2>
            <p className="text-gray-400">Checking Steam, Epic Games and GOG...</p>
          </div>
        )}

        {step === 'found' && (
          <div className="flex flex-col">
            <h2 className="text-2xl font-bold mb-2">
              {candidates.length > 0
                ? `We found ${candidates.length} game${candidates.length === 1 ? '' : 's'} in ${listNames(launchers)}`
                : 'No launcher games found'}
            </h2>
            <p className="text-gray-400 mb-6">
              {discoveryError
                ? `Checking your launchers failed: ${discoveryError}`
                : candidates.length > 0
                  ? "They're in your library and will be tracked whenever you play them."
                  : "That's fine. Scan a folder where you keep games, or add them later from the library."}
            </p>

            {candidates.length > 0 && (
              <div className="max-h-72 overflow-y-auto bg-gray-950/50 rounded-xl border border-gray-800 p-4 mb-6 space-y-4">
                {Object.entries(grouped).map(([launcher, games]) => (
                  <div key={launcher}>
                    <h3 className="text-xs font-semibold uppercase tracking-wider text-gray-500 mb-2">{launcher}</h3>
                    <ul className="grid grid-cols-2 gap-x-4 gap-y-1.5">
                      {games.map((game) => (
                        <li key={game.install_path} className="flex items-center gap-2 text-sm text-gray-300 min-w-0">
                          <Check size={14} className="text-green-400 flex-shrink-0" />
                          <span className="truncate">{game.name}</span>
                        </li>
                      ))}
                    </ul>
                  </div>
                ))}
              </div>
            )}

            <div className="flex justify-between items-center">
              <button
                onClick={() => setShowScan(true)}
                className="flex items-center gap-2 px-5 py-3 bg-gray-800 hover:bg-gray-700 border border-gray-700 text-white font-medium rounded-xl"
              >
                <FolderSearch size={18} /> Scan a folder for other games
              </button>
              <button
                onClick={() => setStep('done')}
                className="px-6 py-3 bg-indigo-600 hover:bg-indigo-500 text-white font-medium rounded-xl"
              >
                Continue
              </button>
            </div>
          </div>
        )}

        {step === 'done' && (
          <div className="flex flex-col items-center text-center">
            <div className="w-20 h-20 bg-green-500/20 rounded-full flex items-center justify-center mb-8 border border-green-500/30">
              <Check size={40} className="text-green-400" />
            </div>
            <h2 className="text-3xl font-bold mb-4">You're all set</h2>
            <p className="text-gray-400 mb-8 max-w-md">
              Play as usual. GameTracker keeps running in the system tray when you close its window.
            </p>
            <div className="w-full max-w-md flex items-center justify-between gap-6 p-4 rounded-xl border border-gray-800 bg-gray-950/50 mb-8 text-left">
              <div>
                <p className="font-medium text-white">Start with Windows</p>
                <p className="text-sm text-gray-400">So games are tracked even if you never open GameTracker.</p>
              </div>
              <Toggle checked={autostart} onChange={changeAutostart} ariaLabel="Toggle starting with Windows" />
            </div>
            <button
              onClick={finish}
              className="px-8 py-4 bg-indigo-600 hover:bg-indigo-500 text-white font-medium rounded-xl transition-all shadow-lg shadow-indigo-600/20"
            >
              Go to my library
            </button>
          </div>
        )}
      </div>

      {showScan && <ScanModal onClose={() => setShowScan(false)} />}
    </div>
  );
}
