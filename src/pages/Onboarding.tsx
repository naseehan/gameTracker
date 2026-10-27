import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { Check, FolderSearch, CheckSquare, Search, ChevronRight } from 'lucide-react';
import { GameCandidate } from '../types';
import { AuthForms } from '../components/AuthForms';
import { useAuthStore } from '../store/authStore';

interface OnboardingProps {
  onComplete: () => void;
}

export function Onboarding({ onComplete }: OnboardingProps) {
  const [step, setStep] = useState(1); // 1 = Welcome/Auth, 2 = AuthForms (optional), 3 = Scan, 4 = Add, 5 = Done
  const [authMode, setAuthMode] = useState<'login' | 'signup'>('signup');
  const [isScanning, setIsScanning] = useState(false);
  const [candidates, setCandidates] = useState<GameCandidate[]>([]);
  const [selectedIndices, setSelectedIndices] = useState<Set<number>>(new Set());
  const [isAdding, setIsAdding] = useState(false);
  const { session } = useAuthStore();

  // Auto-advance past auth step when OAuth login completes via deep link
  useEffect(() => {
    if (session && step === 2) {
      setStep(3);
    }
  }, [session, step]);

  const handleScanFolder = async () => {
    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: 'Select Folder to Scan',
      });
      
      if (selectedPath && typeof selectedPath === 'string') {
        setIsScanning(true);
        const results = await invoke<GameCandidate[]>('scan_folder', { path: selectedPath });
        setCandidates(results);
        setSelectedIndices(new Set(results.map((_, i) => i)));
        setStep(4);
      }
    } catch (err) {
      console.error('Failed to scan folder', err);
    } finally {
      setIsScanning(false);
    }
  };

  const handleAddSelected = async () => {
    setIsAdding(true);
    try {
      for (const idx of selectedIndices) {
        await invoke('add_game', { candidate: candidates[idx] });
      }
      setStep(5);
    } catch (err) {
      console.error('Failed to add games', err);
    } finally {
      setIsAdding(false);
    }
  };

  const handleFinish = async () => {
    try {
      await invoke('complete_onboarding');
      onComplete();
    } catch (err) {
      console.error('Failed to complete onboarding', err);
    }
  };

  const toggleSelection = (index: number) => {
    const newSet = new Set(selectedIndices);
    if (newSet.has(index)) {
      newSet.delete(index);
    } else {
      newSet.add(index);
    }
    setSelectedIndices(newSet);
  };

  return (
    <div className="flex items-center justify-center min-h-screen bg-gray-950 text-gray-100 p-8 w-full z-50 fixed inset-0">
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,_var(--tw-gradient-stops))] from-indigo-900/20 via-gray-950 to-gray-950 -z-10"></div>
      
      <div className="w-full max-w-2xl bg-gray-900/80 backdrop-blur-md border border-gray-800 p-10 rounded-2xl shadow-2xl relative overflow-hidden">
        
        {/* Progress Bar */}
        <div className="absolute top-0 left-0 h-1 bg-gray-800 w-full">
          <div 
            className="h-full bg-indigo-500 transition-all duration-500 ease-out"
            style={{ width: `${(step / 5) * 100}%` }}
          />
        </div>

        {step === 1 && (
          <div className="flex flex-col items-center text-center animate-in fade-in slide-in-from-bottom-4 duration-500">
            <div className="w-16 h-16 bg-gradient-to-br from-indigo-500 to-purple-600 rounded-2xl flex items-center justify-center mb-8 shadow-lg shadow-indigo-500/20">
              <Check size={32} className="text-white" />
            </div>
            <h1 className="text-4xl font-bold mb-4">Welcome to Game Tracker</h1>
            <p className="text-xl text-gray-400 mb-12">Track your games locally and privately.</p>
            
            {session ? (
               <button 
                 onClick={() => setStep(3)}
                 className="px-8 py-4 w-64 mb-4 bg-indigo-600 hover:bg-indigo-500 text-white font-medium rounded-xl transition-all shadow-lg shadow-indigo-600/20 hover:-translate-y-1"
               >
                 Get Started
               </button>
            ) : (
              <div className="flex flex-col space-y-4 w-64">
                <button 
                  onClick={() => { setAuthMode('signup'); setStep(2); }}
                  className="px-8 py-3 bg-indigo-600 hover:bg-indigo-500 text-white font-medium rounded-xl transition-colors shadow-lg shadow-indigo-600/20"
                >
                  Create Account
                </button>
                <button 
                  onClick={() => { setAuthMode('login'); setStep(2); }}
                  className="px-8 py-3 bg-gray-800 hover:bg-gray-700 text-white border border-gray-700 font-medium rounded-xl transition-colors"
                >
                  Sign In
                </button>
                <button 
                  onClick={() => setStep(3)}
                  className="px-8 py-3 text-gray-400 hover:text-white font-medium rounded-xl transition-colors mt-2"
                >
                  Continue Without Account
                </button>
              </div>
            )}
          </div>
        )}

        {step === 2 && (
          <div className="animate-in fade-in duration-300">
            <AuthForms 
              initialMode={authMode} 
              onSuccess={() => setStep(3)} 
              onCancel={() => setStep(1)} 
            />
          </div>
        )}

        {step === 3 && (
          <div className="flex flex-col items-center text-center animate-in fade-in slide-in-from-right-8 duration-500">
            <div className="w-16 h-16 bg-gray-800 rounded-2xl flex items-center justify-center mb-8">
              <FolderSearch size={32} className="text-indigo-400" />
            </div>
            <h2 className="text-2xl font-bold mb-4">Find Your Games</h2>
            <p className="text-gray-400 mb-8 max-w-md mx-auto">
              Select a folder where your games are installed, and we'll automatically detect them. 
              (You can skip this and add them manually later)
            </p>
            
            <div className="flex space-x-4">
              <button 
                onClick={handleScanFolder}
                disabled={isScanning}
                className="px-6 py-3 bg-indigo-600 hover:bg-indigo-500 text-white font-medium rounded-xl transition-all shadow-lg flex items-center disabled:opacity-50"
              >
                {isScanning ? 'Scanning...' : 'Select Folder to Scan'}
              </button>
              <button 
                onClick={() => handleFinish()}
                className="px-6 py-3 bg-gray-800 hover:bg-gray-700 text-white font-medium rounded-xl transition-all"
              >
                Skip for now
              </button>
            </div>
          </div>
        )}

        {step === 4 && (
          <div className="flex flex-col h-full animate-in fade-in slide-in-from-right-8 duration-500">
            <div className="flex items-center space-x-4 mb-6">
              <div className="w-12 h-12 bg-indigo-900/50 rounded-xl flex items-center justify-center">
                <CheckSquare size={24} className="text-indigo-400" />
              </div>
              <div>
                <h2 className="text-2xl font-bold">Select Games to Add</h2>
                <p className="text-gray-400">We found {candidates.length} games</p>
              </div>
            </div>

            <div className="flex-1 overflow-y-auto min-h-[300px] max-h-[400px] bg-gray-950/50 rounded-xl border border-gray-800 p-2 mb-6">
              {candidates.length === 0 ? (
                <div className="h-full flex flex-col items-center justify-center text-gray-500 p-8">
                  <Search size={32} className="mb-2 opacity-50" />
                  <p>No games found in this directory.</p>
                </div>
              ) : (
                <div className="space-y-2">
                  {candidates.map((candidate, idx) => {
                    const isSelected = selectedIndices.has(idx);
                    return (
                      <div 
                        key={idx}
                        onClick={() => toggleSelection(idx)}
                        className={`flex items-center p-3 rounded-lg cursor-pointer transition-colors ${
                          isSelected ? 'bg-indigo-600/20 border border-indigo-500/50' : 'hover:bg-gray-800 border border-transparent'
                        }`}
                      >
                        <div className={`w-5 h-5 rounded flex items-center justify-center mr-4 border ${
                          isSelected ? 'bg-indigo-500 border-indigo-500' : 'border-gray-600'
                        }`}>
                          {isSelected && <Check size={14} className="text-white" />}
                        </div>
                        <div>
                          <p className="font-medium text-white">{candidate.name}</p>
                          <p className="text-xs text-gray-500 truncate max-w-sm">{candidate.executables[0]}</p>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>

            <div className="flex justify-between items-center mt-auto">
              <button 
                onClick={() => setStep(3)}
                className="px-6 py-3 text-gray-400 hover:text-white font-medium"
              >
                Back
              </button>
              <button 
                onClick={handleAddSelected}
                disabled={isAdding || selectedIndices.size === 0}
                className="px-6 py-3 bg-indigo-600 hover:bg-indigo-500 text-white font-medium rounded-xl transition-all shadow-lg flex items-center disabled:opacity-50"
              >
                {isAdding ? 'Adding...' : `Add ${selectedIndices.size} Games`}
                {!isAdding && <ChevronRight size={18} className="ml-2" />}
              </button>
            </div>
          </div>
        )}

        {step === 5 && (
          <div className="flex flex-col items-center text-center animate-in zoom-in duration-500">
            <div className="w-20 h-20 bg-green-500/20 rounded-full flex items-center justify-center mb-8 border border-green-500/30">
              <Check size={40} className="text-green-400" />
            </div>
            <h2 className="text-3xl font-bold mb-4">All Set!</h2>
            <p className="text-gray-400 mb-8 max-w-sm">
              Your games have been added. GameTracker will automatically detect when you play them.
            </p>
            <button 
              onClick={handleFinish}
              className="px-8 py-4 bg-indigo-600 hover:bg-indigo-500 text-white font-medium rounded-xl transition-all shadow-lg shadow-indigo-600/20"
            >
              Go to Library
            </button>
          </div>
        )}

      </div>
    </div>
  );
}
