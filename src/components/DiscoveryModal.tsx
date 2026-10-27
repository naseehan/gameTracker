import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Loader2, CheckCircle2, AlertCircle } from 'lucide-react';
import { useLibraryStore } from '../store/libraryStore';
import { GameCandidate } from '../types';

interface DiscoveryModalProps {
  onClose: () => void;
}

export function DiscoveryModal({ onClose }: DiscoveryModalProps) {
  const [isScanning, setIsScanning] = useState(true);
  const [candidates, setCandidates] = useState<GameCandidate[]>([]);
  const [error, setError] = useState<string | null>(null);
  const { fetchGames } = useLibraryStore();

  useEffect(() => {
    let mounted = true;
    
    async function runDiscovery() {
      try {
        const results = await invoke<GameCandidate[]>('trigger_discovery');
        if (mounted) {
          setCandidates(results);
          setIsScanning(false);
          fetchGames();
        }
      } catch (e) {
        if (mounted) {
          setError(String(e));
          setIsScanning(false);
        }
      }
    }
    
    runDiscovery();
    return () => { mounted = false; };
  }, []);

  const grouped = candidates.reduce((acc, c) => {
    if (!acc[c.source]) acc[c.source] = [];
    acc[c.source].push(c);
    return acc;
  }, {} as Record<string, GameCandidate[]>);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" onClick={!isScanning ? onClose : undefined}>
      <div className="bg-gray-900 border border-gray-800 rounded-xl p-8 max-w-lg w-full shadow-2xl relative max-h-[90vh] flex flex-col" onClick={e => e.stopPropagation()}>
        <h2 className="text-2xl font-bold mb-6 flex-shrink-0">Game Discovery</h2>
        
        <div className="overflow-y-auto flex-1 pr-2">
          {isScanning ? (
            <div className="flex flex-col items-center justify-center py-12">
              <Loader2 className="w-12 h-12 text-indigo-500 animate-spin mb-4" />
              <p className="text-lg font-medium text-gray-200">Scanning configured providers...</p>
              <p className="text-sm text-gray-400 mt-2">Checking Steam, Epic Games, GOG, and local drives.</p>
            </div>
          ) : error ? (
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <AlertCircle className="w-12 h-12 text-red-500 mb-4" />
              <p className="text-lg font-medium text-red-400">Discovery Failed</p>
              <p className="text-sm text-gray-400 mt-2">{error}</p>
            </div>
          ) : (
            <div className="space-y-6">
              <div className="flex flex-col items-center justify-center pb-6 border-b border-gray-800">
                <CheckCircle2 className="w-12 h-12 text-green-500 mb-4" />
                <p className="text-xl font-bold text-gray-200">Discovery Complete</p>
                <p className="text-gray-400 mt-2">Found {candidates.length} game installations.</p>
              </div>

              {Object.entries(grouped).map(([platform, items]) => (
                <div key={platform} className="space-y-3">
                  <h3 className="font-bold text-lg text-white border-b border-gray-700 pb-1">{platform}</h3>
                  <div className="space-y-2 pl-2">
                    {items.map((item, idx) => (
                      <div key={idx} className="flex items-center space-x-2 text-gray-300">
                        <CheckCircle2 className="w-4 h-4 text-green-500" />
                        <span>{item.name}</span>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
              {candidates.length === 0 && (
                <p className="text-gray-500 text-center py-4">No new games were discovered.</p>
              )}
            </div>
          )}
        </div>

        <div className="mt-8 flex justify-end flex-shrink-0 pt-4 border-t border-gray-800">
          <button 
            onClick={onClose}
            disabled={isScanning}
            className={`px-6 py-2 rounded-lg font-medium transition-colors ${isScanning ? 'bg-gray-800 text-gray-500 cursor-not-allowed' : 'bg-indigo-600 hover:bg-indigo-500 text-white shadow-lg shadow-indigo-600/20'}`}
          >
            {isScanning ? 'Scanning...' : 'Done'}
          </button>
        </div>
      </div>
    </div>
  );
}