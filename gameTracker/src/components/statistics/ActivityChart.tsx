import { ResponsiveContainer, BarChart, Bar, XAxis, Tooltip, CartesianGrid } from 'recharts';
import { DayOfWeekActivity } from '../../store/statisticsStore';

interface ActivityChartProps {
  data: DayOfWeekActivity[];
}

export function ActivityChart({ data }: ActivityChartProps) {
  // We want Monday to Sunday typically, but the backend returns them in week order.
  // We just trust the array order provided (Mon, Tue, Wed...)
  
  const formattedData = data.map(d => ({
    day: d.day,
    hours: Number((d.playtime_seconds / 3600).toFixed(2)),
    minutes: Math.floor(d.playtime_seconds / 60)
  }));

  const formatTooltip = (_value: any, _name: any, props: any) => {
    const mins = props.payload.minutes;
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    return [`${h > 0 ? `${h}h ` : ''}${m}m`, 'Playtime'];
  };

  return (
    <div className="bg-gray-900 border border-gray-800 rounded-xl p-5 h-[300px] flex flex-col">
      <h3 className="text-gray-400 font-medium text-sm mb-4">Activity by Day</h3>
      <div className="flex-1 w-full h-full min-h-0">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={formattedData} margin={{ top: 10, right: 10, left: 10, bottom: 0 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#374151" vertical={false} />
            <XAxis 
              dataKey="day" 
              stroke="#9CA3AF" 
              fontSize={12} 
              tickLine={false} 
              axisLine={false}
            />
            <Tooltip 
              contentStyle={{ backgroundColor: '#111827', borderColor: '#374151', borderRadius: '0.5rem' }}
              itemStyle={{ color: '#E5E7EB' }}
              formatter={formatTooltip}
              cursor={{ fill: '#374151', opacity: 0.4 }}
            />
            <Bar dataKey="hours" fill="#10b981" radius={[4, 4, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
