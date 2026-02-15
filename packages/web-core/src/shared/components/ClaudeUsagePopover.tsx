import { Gauge, Loader2 } from 'lucide-react';
import { useMemo } from 'react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
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

    const weekMessages = weekActivity.reduce(
      (s, d) => s + d.messageCount,
      0
    );
    const weekSessions = weekActivity.reduce(
      (s, d) => s + d.sessionCount,
      0
    );
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
      const shortName = name
        .replace('claude-', '')
        .replace(/-\d{8}$/, '');
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

function StatRow({
  label,
  value,
}: {
  label: string;
  value: string | number;
}) {
  return (
    <div className="flex items-center justify-between px-2 py-0.5 text-xs">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-medium tabular-nums">{value}</span>
    </div>
  );
}

export function ClaudeUsagePopover() {
  const { data, isLoading, refresh } = useClaudeUsage();
  const stats = useDerivedStats(data);

  return (
    <DropdownMenu onOpenChange={(open) => open && refresh()}>
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="hidden sm:inline-flex h-6 w-6 ml-1 relative"
                aria-label="Claude Code usage"
              >
                <Gauge className="h-3.5 w-3.5" />
              </Button>
            </DropdownMenuTrigger>
          </TooltipTrigger>
          <TooltipContent side="bottom">Claude Code usage</TooltipContent>
        </Tooltip>
      </TooltipProvider>

      <DropdownMenuContent align="start" className="w-64">
        <DropdownMenuLabel className="text-sm font-semibold">
          Claude Code Usage
        </DropdownMenuLabel>
        <DropdownMenuSeparator />

        {isLoading ? (
          <div className="flex items-center justify-center py-4">
            <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
          </div>
        ) : !stats ? (
          <div className="px-2 py-4 text-sm text-muted-foreground text-center">
            No usage data available
          </div>
        ) : (
          <div className="py-1">
            <DropdownMenuLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Today
            </DropdownMenuLabel>
            <StatRow label="Messages" value={stats.today.messages.toLocaleString()} />
            <StatRow label="Sessions" value={stats.today.sessions} />
            <StatRow label="Tool calls" value={stats.today.toolCalls.toLocaleString()} />
            <StatRow
              label="Output tokens"
              value={formatNumber(stats.today.outputTokens)}
            />

            <DropdownMenuSeparator />

            <DropdownMenuLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Last 7 days
            </DropdownMenuLabel>
            <StatRow label="Messages" value={stats.week.messages.toLocaleString()} />
            <StatRow label="Sessions" value={stats.week.sessions} />
            <StatRow label="Tool calls" value={stats.week.toolCalls.toLocaleString()} />
            <StatRow
              label="Output tokens"
              value={formatNumber(stats.week.outputTokens)}
            />

            <DropdownMenuSeparator />

            <DropdownMenuLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              All time
            </DropdownMenuLabel>
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
                <DropdownMenuSeparator />
                <DropdownMenuLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  By model
                </DropdownMenuLabel>
                {stats.models.map((m) => (
                  <div
                    key={m.name}
                    className="px-2 py-0.5 text-xs flex items-center justify-between"
                  >
                    <span className="text-muted-foreground truncate mr-2">
                      {m.name}
                    </span>
                    <span className="font-medium tabular-nums whitespace-nowrap">
                      {formatNumber(m.outputTokens)} out
                    </span>
                  </div>
                ))}
              </>
            )}
          </div>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
