import { useState } from 'react';
import { supabase } from '../store/authStore';
import { Loader2, AlertCircle, Check } from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';
import { describeError } from '../utils/errors';

interface AuthFormsProps {
  onSuccess: () => void;
  onCancel?: () => void;
  initialMode?: 'login' | 'signup';
}

export function AuthForms({ onSuccess, onCancel, initialMode = 'login' }: AuthFormsProps) {
  const [mode, setMode] = useState<'login' | 'signup' | 'forgot_password'>(initialMode);
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setMessage(null);
    
    if (!email) {
      setError('Please enter a valid email address.');
      return;
    }

    setLoading(true);

    try {
      if (mode === 'signup') {
        if (password.length < 6) {
          setError('Your password does not meet the minimum requirements (at least 6 characters).');
          setLoading(false);
          return;
        }
        if (password !== confirmPassword) {
          setError('Passwords do not match.');
          setLoading(false);
          return;
        }
        const { error: signUpError } = await supabase.auth.signUp({ email, password });
        if (signUpError) throw signUpError;
        onSuccess();
      } else if (mode === 'login') {
        const { error: signInError } = await supabase.auth.signInWithPassword({ email, password });
        if (signInError) {
          if (signInError.message.includes('Invalid login credentials')) {
            throw new Error('Email or password is incorrect.');
          }
          throw signInError;
        }
        onSuccess();
      } else if (mode === 'forgot_password') {
        const { error: resetError } = await supabase.auth.resetPasswordForEmail(email);
        if (resetError) throw resetError;
        setMessage('Password reset email sent. Please check your inbox.');
      }
    } catch (err: any) {
      if (err.message.includes('fetch')) {
        setError('Unable to connect right now. Check your internet connection and try again.');
      } else {
        setError(err.message || 'An unexpected error occurred.');
      }
    } finally {
      setLoading(false);
    }
  };

  const handleGoogleSignIn = async () => {
    try {
      setError(null);
      setLoading(true);
      
      // Start a one-shot local HTTP server to receive the callback
      const port = await invoke<number>('start_oauth_listener');
      const redirectTo = `http://127.0.0.1:${port}/callback`;

      const { data, error } = await supabase.auth.signInWithOAuth({
        provider: 'google',
        options: {
          redirectTo,
          skipBrowserRedirect: true,
        }
      });
      
      if (error) throw error;
      
      if (data?.url) {
        if (data.url.includes('mock-project.supabase.co')) {
          setError('Google Sign-In requires a real Supabase configuration. Please provide a real Supabase URL to enable OAuth.');
        } else {
          await openUrl(data.url);
          setMessage('Please complete the sign-in in your web browser.');
        }
      }
    } catch (err: any) {
      setError('Unable to connect to Google authentication.');
      logError(`Google sign-in failed: ${describeError(err)}`).catch(() => {});
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="w-full max-w-md mx-auto p-6 bg-gray-900 border border-gray-800 rounded-2xl shadow-xl">
      <h2 className="text-2xl font-bold text-white mb-2">
        {mode === 'login' ? 'Sign In' : mode === 'signup' ? 'Create your GameTracker account' : 'Reset Password'}
      </h2>
      <p className="text-gray-400 text-sm mb-6">
        {mode === 'login' && 'Welcome back. Sign in to your account.'}
        {mode === 'signup' && 'Create an account to enable future cloud features.'}
        {mode === 'forgot_password' && 'Enter your email to receive a password reset link.'}
      </p>

      {error && (
        <div className="mb-4 p-3 bg-red-900/30 border border-red-900/50 rounded-lg flex items-start gap-2">
          <AlertCircle size={18} className="text-red-400 flex-shrink-0 mt-0.5" />
          <p className="text-sm text-red-200">{error}</p>
        </div>
      )}

      {message && (
        <div className="mb-4 p-3 bg-green-900/30 border border-green-900/50 rounded-lg flex items-start gap-2">
          <Check size={18} className="text-green-400 flex-shrink-0 mt-0.5" />
          <p className="text-sm text-green-200">{message}</p>
        </div>
      )}

      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-300 mb-1">Email</label>
          <input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
            className="w-full bg-gray-950 border border-gray-800 text-white rounded-lg px-4 py-2.5 focus:ring-2 focus:ring-indigo-500 focus:outline-none"
            placeholder="you@example.com"
          />
        </div>

        {mode !== 'forgot_password' && (
          <div>
            <label className="block text-sm font-medium text-gray-300 mb-1">Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              className="w-full bg-gray-950 border border-gray-800 text-white rounded-lg px-4 py-2.5 focus:ring-2 focus:ring-indigo-500 focus:outline-none"
              placeholder="••••••••"
            />
          </div>
        )}

        {mode === 'signup' && (
          <div>
            <label className="block text-sm font-medium text-gray-300 mb-1">Confirm Password</label>
            <input
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              required
              className="w-full bg-gray-950 border border-gray-800 text-white rounded-lg px-4 py-2.5 focus:ring-2 focus:ring-indigo-500 focus:outline-none"
              placeholder="••••••••"
            />
          </div>
        )}

        <button
          type="submit"
          disabled={loading}
          className="w-full flex items-center justify-center py-2.5 bg-indigo-600 hover:bg-indigo-700 text-white font-medium rounded-lg transition-colors disabled:opacity-50"
        >
          {loading ? <Loader2 className="animate-spin" size={20} /> : (mode === 'login' ? 'Sign In' : mode === 'signup' ? 'Create Account' : 'Send Reset Link')}
        </button>
      </form>

      {mode !== 'forgot_password' && (
        <>
          <div className="flex items-center my-6">
            <div className="flex-grow border-t border-gray-800"></div>
            <span className="px-3 text-xs text-gray-500 font-medium uppercase">or</span>
            <div className="flex-grow border-t border-gray-800"></div>
          </div>
          
          <button
            onClick={handleGoogleSignIn}
            disabled={loading}
            className="w-full flex items-center justify-center space-x-2 py-2.5 bg-gray-800 hover:bg-gray-700 text-white font-medium rounded-lg transition-colors border border-gray-700 disabled:opacity-50"
          >
            <svg className="w-5 h-5" viewBox="0 0 24 24">
              <path fill="currentColor" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" />
              <path fill="currentColor" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" />
              <path fill="currentColor" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" />
              <path fill="currentColor" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" />
            </svg>
            <span>Continue with Google</span>
          </button>
        </>
      )}

      <div className="mt-6 flex flex-col items-center space-y-2 text-sm">
        {mode === 'login' && (
          <>
            <button onClick={() => setMode('forgot_password')} className="text-indigo-400 hover:text-indigo-300">Forgot your password?</button>
            <div className="text-gray-400">
              Don't have an account? <button onClick={() => setMode('signup')} className="text-white hover:underline">Create account</button>
            </div>
          </>
        )}
        {mode === 'signup' && (
          <div className="text-gray-400">
            Already have an account? <button onClick={() => setMode('login')} className="text-white hover:underline">Sign In</button>
          </div>
        )}
        {mode === 'forgot_password' && (
          <button onClick={() => setMode('login')} className="text-white hover:underline">Return to Sign In</button>
        )}

        {onCancel && (
          <button onClick={onCancel} className="mt-4 px-4 py-2 text-gray-500 hover:text-gray-300 transition-colors">
            Cancel
          </button>
        )}
      </div>
    </div>
  );
}
