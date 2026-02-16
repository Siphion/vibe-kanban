import { GaugeIcon, SpinnerIcon } from '@phosphor-icons/react';
import { useMemo } from 'react';
import { cn } from '@/lib/utils';
import {
  Popover,
  PopoverTrigger,
  PopoverContent,
} from '../primitives/Popover';
import { Tooltip } from '../primitives/Tooltip';
import { useClaudeUsage, type ClaudeStatsCache } from '@/hooks/useClaudeUsage';

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

function getToday(): string {
  return new Date().toISOString().slice(0, 10);
}

function getLast7DaysRange(): [string, string] {
  const now = new Date();
  const end = now.toISOString().slice(0, 10);
  const start = new Date(now);
  start.setDate(start.getDate() - 6);
  return [start.toISOString().slice(0, 10), end];
}

function useDerivedStats(data: ClaudeStatsCache | null) {
  return useMemo(() => {
    if (!data) return null;

    const today = getToday();
    const [weekStart, weekEnd] = getLast7DaysRange();

    const todayActivity = data.dailyActivity.find((d) => d.date === today);
    const todayTokens = data.dailyModelTokens.find((d) => d.date === today);

    const weekActivity = data.dailyActivity.filter(
      (d) => d.date >= weekStart && d.date <= weekEnd
    );
    const weekTokens = data.dailyModelTokens.filter(
      (d) => d.date >= weekStart && d.date <= weekEnd
    );

    const weekMessages = weekActivity.reduce((s, d) => s + d.messageCount, 0);
    const weekSessions = weekActivity.reduce((s, d) => s + d.sessionCount, 0);
    const weekToolCalls = weekActivity.reduce(
      (s, d) => s + d.toolCallCount,
      0
    );
    const weekOutputTokens = weekTokens.reduce(
      (s, d) =>
        s + Object.values(d.tokensByModel).reduce((a, b) => a + b, 0),
      0
    );

    const todayOutputTokens = todayTokens
      ? Object.values(todayTokens.tokensByModel).reduce((a, b) => a + b, 0)
      : 0;

    const models = Object.entries(data.modelUsage).map(([name, usage]) => {
      const shortName = name.replace('claude-', '').replace(/-\d{8}$/, '');
      return {
        name: shortName,
        outputTokens: usage.outputTokens,
        inputTokens: usage.inputTokens,
      };
    });

    return {
      today: {
        messages: todayActivity?.messageCount ?? 0,
        sessions: todayActivity?.sessionCount ?? 0,
        toolCalls: todayActivity?.toolCallCount ?? 0,
        outputTokens: todayOutputTokens,
      },
      week: {
        messages: weekMessages,
        sessions: weekSessions,
        toolCalls: weekToolCalls,
        outputTokens: weekOutputTokens,
      },
      totals: {
        sessions: data.totalSessions,
        messages: data.totalMessages,
      },
      models,
    };
  }, [data]);
}

function StatRow({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="flex items-center justify-between py-0.5 text-xs">
      <span className="text-low">{label}</span>
      <span className="font-medium tabular-nums text-normal">{value}</span>
    </div>
  );
}

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <div className="text-[10px] font-semibold uppercase tracking-wide text-low mt-2 mb-0.5">
      {children}
    </div>
  );
}

export function AppBarClaudeUsage() {
  const { data, isLoading, refresh } = useClaudeUsage();
  const stats = useDerivedStats(data);

  return (
    <Popover onOpenChange={(open: boolean) => open && refresh()}>
      <Tooltip content="Claude Code usage" side="right">
        <PopoverTrigger asChild>
          <button
            type="button"
            className={cn(
              'flex items-center justify-center w-10 h-10 rounded-lg',
              'transition-colors cursor-pointer',
              'focus:outline-none focus-visible:ring-2 focus-visible:ring-brand',
              'bg-primary text-normal hover:bg-brand/10'
            )}
            aria-label="Claude Code usage"
          >
            <GaugeIcon className="size-icon-base" weight="bold" />
          </button>
        </PopoverTrigger>
      </Tooltip>

      <PopoverContent side="right" sideOffset={8} className="w-64 p-3">
        <p className="text-sm font-semibold text-high">Claude Code Usage</p>

        <div className="mt-2 h-px bg-border" />

        {isLoading ? (
          <div className="flex items-center justify-center py-4">
            <SpinnerIcon className="size-4 animate-spin text-low" />
          </div>
        ) : !stats ? (
          <div className="py-4 text-sm text-low text-center">
            No usage data available
          </div>
        ) : (
          <div>
            <SectionLabel>Today</SectionLabel>
            <StatRow
              label="Messages"
              value={stats.today.messages.toLocaleString()}
            />
            <StatRow label="Sessions" value={stats.today.sessions} />
            <StatRow
              label="Tool calls"
              value={stats.today.toolCalls.toLocaleString()}
            />
            <StatRow
              label="Output tokens"
              value={formatNumber(stats.today.outputTokens)}
            />

            <div className="mt-1.5 h-px bg-border" />

            <SectionLabel>Last 7 days</SectionLabel>
            <StatRow
              label="Messages"
              value={stats.week.messages.toLocaleString()}
            />
            <StatRow label="Sessions" value={stats.week.sessions} />
            <StatRow
              label="Tool calls"
              value={stats.week.toolCalls.toLocaleString()}
            />
            <StatRow
              label="Output tokens"
              value={formatNumber(stats.week.outputTokens)}
            />

            <div className="mt-1.5 h-px bg-border" />

            <SectionLabel>All time</SectionLabel>
            <StatRow
              label="Sessions"
              value={stats.totals.sessions.toLocaleString()}
            />
            <StatRow
              label="Messages"
              value={stats.totals.messages.toLocaleString()}
            />

            {stats.models.length > 0 && (
              <>
                <div className="mt-1.5 h-px bg-border" />
                <SectionLabel>By model</SectionLabel>
                {stats.models.map((m) => (
                  <div
                    key={m.name}
                    className="py-0.5 text-xs flex items-center justify-between"
                  >
                    <span className="text-low truncate mr-2">{m.name}</span>
                    <span className="font-medium tabular-nums text-normal whitespace-nowrap">
                      {formatNumber(m.outputTokens)} out
                    </span>
                  </div>
                ))}
              </>
            )}
          </div>
        )}
      </PopoverContent>
    </Popover>
  );
}
