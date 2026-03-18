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

export interface UsageWindow {
  utilization: number;
  resetsAt: string;
}

export interface ExtraUsage {
  isEnabled: boolean;
  monthlyLimit: number;
  usedCredits: number;
  utilization: number;
}

export interface LiveUsageData {
  fiveHour: UsageWindow | null;
  sevenDay: UsageWindow | null;
  sevenDayOpus: UsageWindow | null;
  sevenDaySonnet: UsageWindow | null;
  extraUsage: ExtraUsage | null;
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
      setData(stats);
      try {
        const live = await handleApiResponse<LiveUsageData>(liveRes);
        setLiveData(live);
      } catch {
        // Live data may fail (no keychain, no OAuth) — non-blocking
        setLiveData(null);
      }
    } catch (err) {
      console.error('Failed to fetch Claude usage:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  return { data, liveData, isLoading, refresh };
}
