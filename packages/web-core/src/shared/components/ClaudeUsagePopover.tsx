import { Gauge, Loader2 } from 'lucide-react';
import { useMemo, useState } from 'react';
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
import {
  useClaudeUsage,
  type ClaudeStatsCache,
  type LiveUsageData,
} from '@/hooks/useClaudeUsage';
import {
  CLAUDE_PLANS,
  getSelectedPlanId,
  setSelectedPlanId,
  getPlanById,
} from '@/config/claude-plans';

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

function formatTimeUntil(isoString: string): string {
  const reset = new Date(isoString).getTime();
  const now = Date.now();
  const diffMs = reset - now;
  if (diffMs <= 0) return 'now';
  const hours = Math.floor(diffMs / 3_600_000);
  const mins = Math.floor((diffMs % 3_600_000) / 60_000);
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}

function formatTime(isoString: string): string {
  return new Date(isoString).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
  });
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

    const today = new Date().toISOString().slice(0, 10);
    const [weekStart, weekEnd] = getLast7DaysRange();

    const todayActivity = data.dailyActivity.find((d) => d.date === today);

    const weekActivity = data.dailyActivity.filter(
      (d) => d.date >= weekStart && d.date <= weekEnd
    );

    const weekSessions = weekActivity.reduce(
      (s, d) => s + d.sessionCount,
      0
    );

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
        sessions: todayActivity?.sessionCount ?? 0,
        toolCalls: todayActivity?.toolCallCount ?? 0,
      },
      week: { sessions: weekSessions },
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
  pct,
  sub,
}: {
  label: string;
  value: string | number;
  pct?: number;
  sub?: string;
}) {
  return (
    <div className="px-2 py-0.5">
      <div className="flex items-center justify-between text-xs">
        <span className="text-muted-foreground">{label}</span>
        <span className="font-medium tabular-nums">
          {value}
          {pct != null && (
            <span className="text-muted-foreground ml-1">
              ({Math.round(pct)}%)
            </span>
          )}
        </span>
      </div>
      {sub && (
        <div className="px-0 text-[10px] text-muted-foreground mt-0.5">
          {sub}
        </div>
      )}
      {pct != null && (
        <div className="h-1 w-full rounded-full bg-muted mt-0.5">
          <div
            className={`h-full rounded-full transition-all ${
              Math.min(pct, 100) >= 90
                ? 'bg-destructive'
                : Math.min(pct, 100) >= 70
                  ? 'bg-yellow-500'
                  : 'bg-primary'
            }`}
            style={{ width: `${Math.min(pct, 100)}%` }}
          />
        </div>
      )}
    </div>
  );
}

export function ClaudeUsagePopover() {
  const { data, liveData, isLoading, refresh } = useClaudeUsage();
  const stats = useDerivedStats(data);
  const [planId, setPlanId] = useState(getSelectedPlanId);
  const plan = getPlanById(planId) ?? CLAUDE_PLANS[1];

  const handlePlanChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const id = e.target.value;
    setPlanId(id);
    setSelectedPlanId(id);
  };

  const windowPct = liveData
    ? (liveData.windowMessages / plan.messagesPer5h) * 100
    : 0;
  const todayPct = liveData
    ? (liveData.todayMessages / plan.messagesPerDay) * 100
    : 0;
  const weekPct = liveData
    ? (liveData.weekMessages / (plan.messagesPerDay * 7)) * 100
    : 0;

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

      <DropdownMenuContent align="start" className="w-72">
        <DropdownMenuLabel className="text-sm font-semibold">
          Claude Code Usage
        </DropdownMenuLabel>

        {/* Plan selector */}
        <div className="px-2 pb-1">
          <select
            value={planId}
            onChange={handlePlanChange}
            className="w-full rounded-md px-2 py-1 text-xs bg-muted border border-border focus:outline-none focus:ring-1 focus:ring-primary cursor-pointer"
          >
            {CLAUDE_PLANS.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        </div>

        <DropdownMenuSeparator />

        {isLoading ? (
          <div className="flex items-center justify-center py-4">
            <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
          </div>
        ) : !liveData ? (
          <div className="px-2 py-4 text-sm text-muted-foreground text-center">
            No usage data available
          </div>
        ) : (
          <div className="py-1">
            {/* 5-hour rolling window */}
            <DropdownMenuLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              5h window
            </DropdownMenuLabel>
            <StatRow
              label="Messages"
              value={`${liveData.windowMessages.toLocaleString()} / ${plan.messagesPer5h.toLocaleString()}`}
              pct={windowPct}
              sub={
                liveData.windowReset
                  ? `Resets in ${formatTimeUntil(liveData.windowReset)} (${formatTime(liveData.windowReset)})`
                  : liveData.windowMessages === 0
                    ? 'No messages in current window'
                    : undefined
              }
            />
            <StatRow
              label="Output tokens"
              value={formatNumber(liveData.windowOutputTokens)}
            />

            <DropdownMenuSeparator />

            {/* Today */}
            <DropdownMenuLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Today
            </DropdownMenuLabel>
            <StatRow
              label="Messages"
              value={`${liveData.todayMessages.toLocaleString()} / ${plan.messagesPerDay.toLocaleString()}`}
              pct={todayPct}
            />
            {stats && (
              <>
                <StatRow label="Sessions" value={stats.today.sessions} />
                <StatRow
                  label="Tool calls"
                  value={stats.today.toolCalls.toLocaleString()}
                />
              </>
            )}
            <StatRow
              label="Output tokens"
              value={formatNumber(liveData.todayOutputTokens)}
            />

            <DropdownMenuSeparator />

            {/* Week */}
            <DropdownMenuLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Last 7 days
            </DropdownMenuLabel>
            <StatRow
              label="Messages"
              value={`${liveData.weekMessages.toLocaleString()} / ${(plan.messagesPerDay * 7).toLocaleString()}`}
              pct={weekPct}
            />
            {stats && (
              <StatRow label="Sessions" value={stats.week.sessions} />
            )}

            {/* Active sessions in window */}
            {liveData.sessions.length > 0 && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  Sessions in window
                </DropdownMenuLabel>
                {liveData.sessions.map((s) => {
                  const project = s.project.split('/').pop() ?? s.project;
                  return (
                    <div
                      key={s.sessionId}
                      className="px-2 py-0.5 text-xs flex items-center justify-between"
                    >
                      <span
                        className="text-muted-foreground truncate mr-2"
                        title={s.project}
                      >
                        {project}
                      </span>
                      <span className="font-medium tabular-nums whitespace-nowrap">
                        {s.messages} msg
                      </span>
                    </div>
                  );
                })}
              </>
            )}

            {/* Models */}
            {stats && stats.models.length > 0 && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  By model (all time)
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

            <DropdownMenuSeparator />
            <div className="px-2 py-1">
              <p className="text-[10px] text-muted-foreground leading-tight">
                Limits are estimates (~{plan.messagesPer5h} msg/5h window, ~3
                windows/day). Edit config/claude-plans.ts to update.
              </p>
            </div>
          </div>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
