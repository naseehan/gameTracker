import { ResponsiveContainer, BarChart, Bar, XAxis, YAxis, Tooltip, CartesianGrid } from 'recharts';
import { DailyPlaytime } from '../../store/statisticsStore';

interface PlaytimeChartProps {
  data: DailyPlaytime[];
}

export function PlaytimeChart({ data }: PlaytimeChartProps) {
  // Format data for display: convert seconds to hours for Y-axis
  const formattedData = data.map(d => ({
    ...d,
    hours: Number((d.playtime_seconds / 3600).toFixed(2)),
    minutes: Math.floor(d.playtime_seconds / 60)
  }));

  const formatTooltip = (_value: any, _name: any, props: any) => {
    const mins = props.payload.minutes;
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    return [`${h > 0 ? `${h}h ` : ''}${m}m`, 'Playtime'];
  };

  const formatXAxis = (tickItem: string) => {
    // If it's a date like 2023-10-01, simplify it
    if (tickItem.includes('-')) {
      const parts = tickItem.split('-');
      if (parts.length === 3) {
        return `${parts[1]}/${parts[2]}`;
      }
    }
    return tickItem;
  };

  return (
    <div className="bg-gray-900 border border-gray-800 rounded-xl p-5 h-[300px] flex flex-col">
      <h3 className="text-gray-400 font-medium text-sm mb-4">Playtime Over Time</h3>
      <div className="flex-1 w-full h-full min-h-0">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={formattedData} margin={{ top: 10, right: 10, left: -20, bottom: 0 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#374151" vertical={false} />
            <XAxis 
              dataKey="date_label" 
              stroke="#9CA3AF" 
              fontSize={12} 
              tickLine={false} 
              axisLine={false}
              tickFormatter={formatXAxis} 
            />
            <YAxis 
              stroke="#9CA3AF" 
              fontSize={12} 
              tickLine={false} 
              axisLine={false}
              tickFormatter={(val) => `${val}h`}
            />
            <Tooltip 
              contentStyle={{ backgroundColor: '#111827', borderColor: '#374151', borderRadius: '0.5rem' }}
              itemStyle={{ color: '#E5E7EB' }}
              formatter={formatTooltip}
              cursor={{ fill: '#374151', opacity: 0.4 }}
            />
            <Bar dataKey="hours" fill="#6366f1" radius={[4, 4, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
