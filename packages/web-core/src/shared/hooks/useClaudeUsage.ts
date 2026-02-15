import { useState, useCallback } from 'react';
import { handleApiResponse } from '@/lib/api';

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

export function useClaudeUsage() {
  const [data, setData] = useState<ClaudeStatsCache | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const response = await fetch('/api/claude-usage');
      const result = await handleApiResponse<ClaudeStatsCache>(response);
      setData(result);
    } catch (err) {
      console.error('Failed to fetch Claude usage:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  return { data, isLoading, refresh };
}
