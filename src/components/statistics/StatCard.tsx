import { ReactNode } from 'react';

interface StatCardProps {
  title: string;
  value: ReactNode;
  subtitle?: string;
  icon?: ReactNode;
}

export function StatCard({ title, value, subtitle, icon }: StatCardProps) {
  return (
    <div className="bg-gray-900 border border-gray-800 rounded-xl p-5 flex flex-col justify-between">
      <div className="flex justify-between items-start mb-2">
        <h3 className="text-gray-400 font-medium text-sm">{title}</h3>
        {icon && <div className="text-indigo-400">{icon}</div>}
      </div>
      <div>
        <div className="text-2xl font-bold text-white tracking-wide">{value}</div>
        {subtitle && <div className="text-xs text-gray-500 mt-1">{subtitle}</div>}
      </div>
    </div>
  );
}
