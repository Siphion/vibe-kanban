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
import { useClaudeUsage } from '@/hooks/useClaudeUsage';
import type { ClaudeStatsCache } from '@/hooks/useClaudeUsage';

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

function useDerivedStats(data: ClaudeStatsCache | null) {
  return useMemo(() => {
    if (!data) return null;
    const models = Object.entries(data.modelUsage).map(([name, usage]) => {
      const shortName = name.replace('claude-', '').replace(/-\d{8}$/, '');
      return { name: shortName, outputTokens: usage.outputTokens };
    });
    return {
      totals: { sessions: data.totalSessions, messages: data.totalMessages },
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
        <div className="text-[10px] text-muted-foreground mt-0.5">{sub}</div>
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
        <DropdownMenuSeparator />

        {isLoading ? (
          <div className="flex items-center justify-center py-4">
            <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
          </div>
        ) : (
          <div className="py-1">
            {liveData?.fiveHour && (
              <>
                <DropdownMenuLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  Session (5h window)
                </DropdownMenuLabel>
                <StatRow
                  label="Utilization"
                  value={`${liveData.fiveHour.utilization}%`}
                  pct={liveData.fiveHour.utilization}
                  sub={`Resets in ${formatTimeUntil(liveData.fiveHour.resetsAt)} (${formatTime(liveData.fiveHour.resetsAt)})`}
                />
              </>
            )}

            {liveData?.sevenDay && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  Weekly (7 days)
                </DropdownMenuLabel>
                <StatRow
                  label="Utilization"
                  value={`${liveData.sevenDay.utilization}%`}
                  pct={liveData.sevenDay.utilization}
                  sub={`Resets in ${formatTimeUntil(liveData.sevenDay.resetsAt)} (${new Date(liveData.sevenDay.resetsAt).toLocaleDateString([], { weekday: 'short', day: 'numeric', month: 'short' })})`}
                />
              </>
            )}

            {(liveData?.sevenDayOpus || liveData?.sevenDaySonnet) && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  By model (7 days)
                </DropdownMenuLabel>
                {liveData?.sevenDayOpus && (
                  <StatRow
                    label="Opus"
                    value={`${liveData.sevenDayOpus.utilization}%`}
                    pct={liveData.sevenDayOpus.utilization}
                  />
                )}
                {liveData?.sevenDaySonnet && (
                  <StatRow
                    label="Sonnet"
                    value={`${liveData.sevenDaySonnet.utilization}%`}
                    pct={liveData.sevenDaySonnet.utilization}
                  />
                )}
              </>
            )}

            {liveData?.extraUsage?.isEnabled && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  Extra usage
                </DropdownMenuLabel>
                <StatRow
                  label="Credits"
                  value={`$${(liveData.extraUsage.usedCredits / 100).toFixed(2)} / $${(liveData.extraUsage.monthlyLimit / 100).toFixed(2)}`}
                  pct={liveData.extraUsage.utilization}
                />
              </>
            )}

            {!liveData && (
              <div className="px-2 py-2 text-xs text-muted-foreground text-center">
                Live usage unavailable
              </div>
            )}

            {stats && (
              <>
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
              </>
            )}

            {stats && stats.models.length > 0 && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuLabel className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  Output tokens (all time)
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
                      {formatNumber(m.outputTokens)}
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
