import {
  BarChart, Bar, LineChart, Line, XAxis, YAxis, CartesianGrid,
  Tooltip, ResponsiveContainer, Legend,
} from 'recharts'
import { format } from 'date-fns'
import type { HistogramBucket, TokenBucket } from '../lib/api'

interface RequestChartProps {
  buckets: HistogramBucket[]
  bucketSecs: number
}

export function RequestChart({ buckets, bucketSecs }: RequestChartProps) {
  const data = buckets.map(b => ({
    time: format(new Date(b.timestamp), bucketSecs < 3600 ? 'HH:mm' : 'MM/dd HH:mm'),
    success: b.success,
    error: b.error,
  }))

  return (
    <ResponsiveContainer width="100%" height={220}>
      <BarChart data={data} barSize={6} barGap={2}>
        <CartesianGrid strokeDasharray="3 3" stroke="#1f2937" />
        <XAxis dataKey="time" tick={{ fill: '#6b7280', fontSize: 11 }} />
        <YAxis tick={{ fill: '#6b7280', fontSize: 11 }} />
        <Tooltip
          contentStyle={{ background: '#111827', border: '1px solid #374151', borderRadius: 8 }}
          labelStyle={{ color: '#9ca3af' }}
        />
        <Legend wrapperStyle={{ fontSize: 12, color: '#9ca3af' }} />
        <Bar dataKey="success" fill="#22c55e" name="Success" radius={[2, 2, 0, 0]} />
        <Bar dataKey="error"   fill="#ef4444" name="Error"   radius={[2, 2, 0, 0]} />
      </BarChart>
    </ResponsiveContainer>
  )
}

interface TokenChartProps {
  buckets: TokenBucket[]
  bucketSecs: number
}

export function TokenChart({ buckets, bucketSecs }: TokenChartProps) {
  const data = buckets.map(b => ({
    time: format(new Date(b.timestamp), bucketSecs < 3600 ? 'HH:mm' : 'MM/dd HH:mm'),
    prompt: b.prompt_tokens,
    completion: b.completion_tokens,
  }))

  return (
    <ResponsiveContainer width="100%" height={220}>
      <LineChart data={data}>
        <CartesianGrid strokeDasharray="3 3" stroke="#1f2937" />
        <XAxis dataKey="time" tick={{ fill: '#6b7280', fontSize: 11 }} />
        <YAxis tick={{ fill: '#6b7280', fontSize: 11 }} />
        <Tooltip
          contentStyle={{ background: '#111827', border: '1px solid #374151', borderRadius: 8 }}
          labelStyle={{ color: '#9ca3af' }}
        />
        <Legend wrapperStyle={{ fontSize: 12, color: '#9ca3af' }} />
        <Line dataKey="prompt"     stroke="#3b82f6" name="Prompt"     dot={false} strokeWidth={2} />
        <Line dataKey="completion" stroke="#8b5cf6" name="Completion" dot={false} strokeWidth={2} />
      </LineChart>
    </ResponsiveContainer>
  )
}
