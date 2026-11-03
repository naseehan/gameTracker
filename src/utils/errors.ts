import { error as logError } from '@tauri-apps/plugin-log';
import { toast } from '../store/toastStore';

/** The message to show for an error from a backend command or a thrown exception. */
export function describeError(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err instanceof Error) return err.message;
  return String(err);
}

/** Shows an error to the user and writes it to the log file. */
export function reportError(context: string, err: unknown) {
  const message = describeError(err);
  toast.error(`${context}: ${message}`);
  logError(`${context}: ${message}`).catch(() => {});
}
