import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Loader2, CheckCircle2, AlertCircle } from 'lucide-react';
import { GameCandidate } from '../types';
import { Modal } from './Modal';
import { describeError } from '../utils/errors';

interface DiscoveryModalProps {
  onClose: () => void;
}

export function groupBySource(candidates: GameCandidate[]): Record<string, GameCandidate[]> {
  return candidates.reduce((acc, c) => {
    (acc[c.source] ??= []).push(c);
    return acc;
  }, {} as Record<string, GameCandidate[]>);
}

export function DiscoveryModal({ onClose }: DiscoveryModalProps) {
  const [isScanning, setIsScanning] = useState(true);
  const [candidates, setCandidates] = useState<GameCandidate[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    invoke<GameCandidate[]>('trigger_discovery')
      .then((results) => mounted && setCandidates(results))
      .catch((e) => mounted && setError(describeError(e)))
      .finally(() => mounted && setIsScanning(false));
    return () => { mounted = false; };
  }, []);

  const grouped = groupBySource(candidates);

  return (
    <Modal title="Find Games" onClose={onClose} width="max-w-lg" locked={isScanning}>
      {isScanning ? (
        <div className="flex flex-col items-center justify-center py-12">
          <Loader2 className="w-12 h-12 text-indigo-500 animate-spin mb-4" />
          <p className="text-lg font-medium text-gray-200">Checking your game launchers...</p>
          <p className="text-sm text-gray-400 mt-2">Steam, Epic Games and GOG, if they're installed and enabled in Settings.</p>
        </div>
      ) : error ? (
        <div className="flex flex-col items-center justify-center py-12 text-center">
          <AlertCircle className="w-12 h-12 text-red-500 mb-4" />
          <p className="text-lg font-medium text-red-400">Couldn't check for games</p>
          <p className="text-sm text-gray-400 mt-2">{error}</p>
        </div>
      ) : (
        <div className="space-y-6">
          <div className="flex flex-col items-center justify-center pb-6 border-b border-gray-800 text-center">
            <CheckCircle2 className="w-12 h-12 text-green-500 mb-4" />
            <p className="text-xl font-bold text-gray-200">
              {candidates.length === 0 ? 'No installed games found' : `${candidates.length} installed games found`}
            </p>
            <p className="text-gray-400 mt-2 text-sm">
              {candidates.length === 0
                ? 'Use Scan Folder or Add Game for games from other places.'
                : 'They are all in your library and are tracked when you play them.'}
            </p>
          </div>

          {Object.entries(grouped).map(([platform, items]) => (
            <div key={platform} className="space-y-3">
              <h3 className="font-bold text-lg text-white border-b border-gray-700 pb-1">{platform}</h3>
              <ul className="space-y-2 pl-2">
                {items.map((item) => (
                  <li key={item.install_path} className="flex items-center space-x-2 text-gray-300">
                    <CheckCircle2 className="w-4 h-4 text-green-500 flex-shrink-0" />
                    <span>{item.name}</span>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
      )}

      <div className="mt-8 flex justify-end pt-4 border-t border-gray-800">
        <button
          onClick={onClose}
          disabled={isScanning}
          className="px-6 py-2 rounded-lg font-medium transition-colors bg-indigo-600 hover:bg-indigo-500 text-white disabled:bg-gray-800 disabled:text-gray-500"
        >
          {isScanning ? 'Checking...' : 'Done'}
        </button>
      </div>
    </Modal>
  );
}
