import { useState, useCallback } from 'react';
import { handleApiResponse } from '@/shared/lib/api';

export interface DailyActivity {
  date: string;
  messageCount: number;
  sessionCount: number;
  toolCallCount: number;
}

export interface DailyModelTokens {
  date: string;
  tokensByModel: Record<string, number>;
}

export interface ModelUsageEntry {
  inputTokens: number;
  outputTokens: number;
  cacheReadInputTokens: number;
  cacheCreationInputTokens: number;
  costUSD: number;
}

export interface ClaudeStatsCache {
  version: number;
  lastComputedDate: string;
  dailyActivity: DailyActivity[];
  dailyModelTokens: DailyModelTokens[];
  modelUsage: Record<string, ModelUsageEntry>;
  totalSessions: number;
  totalMessages: number;
  hourCounts: Record<string, number>;
}

export interface LiveSessionUsage {
  sessionId: string;
  project: string;
  startedAt: string;
  messages: number;
  outputTokens: number;
}

export interface LiveUsageData {
  windowMessages: number;
  windowStart: string | null;
  windowReset: string | null;
  sessions: LiveSessionUsage[];
  todayMessages: number;
  weekMessages: number;
  windowOutputTokens: number;
  todayOutputTokens: number;
}

export function useClaudeUsage() {
  const [data, setData] = useState<ClaudeStatsCache | null>(null);
  const [liveData, setLiveData] = useState<LiveUsageData | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const [statsRes, liveRes] = await Promise.all([
        fetch('/api/claude-usage'),
        fetch('/api/claude-usage/live'),
      ]);
      const stats = await handleApiResponse<ClaudeStatsCache>(statsRes);
      const live = await handleApiResponse<LiveUsageData>(liveRes);
      setData(stats);
      setLiveData(live);
    } catch (err) {
      console.error('Failed to fetch Claude usage:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  return { data, liveData, isLoading, refresh };
}
