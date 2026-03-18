import { GaugeIcon, SpinnerIcon } from '@phosphor-icons/react';
import { useMemo } from 'react';
import { cn } from '../lib/cn';
import {
  Popover,
  PopoverTrigger,
  PopoverContent,
} from './Popover';
import { Tooltip } from './Tooltip';
import {
  useClaudeUsage,
} from '@/hooks/useClaudeUsage';
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
      return {
        name: shortName,
        outputTokens: usage.outputTokens,
        inputTokens: usage.inputTokens,
      };
    });

    return {
      totals: {
        sessions: data.totalSessions,
        messages: data.totalMessages,
      },
      models,
    };
  }, [data]);
}

function ProgressBar({ pct }: { pct: number }) {
  const clamped = Math.min(pct, 100);
  const color =
    clamped >= 90
      ? 'bg-red-500'
      : clamped >= 70
        ? 'bg-amber-500'
        : 'bg-brand';
  return (
    <div className="h-1 w-full rounded-full bg-surface-secondary mt-0.5">
      <div
        className={cn('h-full rounded-full transition-all', color)}
        style={{ width: `${clamped}%` }}
      />
    </div>
  );
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
    <div className="py-0.5">
      <div className="flex items-center justify-between text-xs">
        <span className="text-low">{label}</span>
        <span className="font-medium tabular-nums text-normal">
          {value}
          {pct != null && (
            <span className="text-low ml-1">({Math.round(pct)}%)</span>
          )}
        </span>
      </div>
      {sub && <div className="text-[10px] text-low mt-0.5">{sub}</div>}
      {pct != null && <ProgressBar pct={pct} />}
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
  const { data, liveData, isLoading, refresh } = useClaudeUsage();
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

      <PopoverContent side="right" sideOffset={8} className="w-72 p-3">
        <p className="text-sm font-semibold text-high">Claude Code Usage</p>

        <div className="mt-2 h-px bg-border" />

        {isLoading ? (
          <div className="flex items-center justify-center py-4">
            <SpinnerIcon className="size-4 animate-spin text-low" />
          </div>
        ) : (
          <div>
            {/* Live usage from Anthropic API */}
            {liveData?.fiveHour && (
              <>
                <SectionLabel>Session (5h window)</SectionLabel>
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
                <div className="mt-1.5 h-px bg-border" />
                <SectionLabel>Weekly (7 days)</SectionLabel>
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
                <div className="mt-1.5 h-px bg-border" />
                <SectionLabel>By model (7 days)</SectionLabel>
                {liveData.sevenDayOpus && (
                  <StatRow
                    label="Opus"
                    value={`${liveData.sevenDayOpus.utilization}%`}
                    pct={liveData.sevenDayOpus.utilization}
                  />
                )}
                {liveData.sevenDaySonnet && (
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
                <div className="mt-1.5 h-px bg-border" />
                <SectionLabel>Extra usage</SectionLabel>
                <StatRow
                  label="Credits"
                  value={`$${(liveData.extraUsage.usedCredits / 100).toFixed(2)} / $${(liveData.extraUsage.monthlyLimit / 100).toFixed(2)}`}
                  pct={liveData.extraUsage.utilization}
                />
              </>
            )}

            {!liveData && (
              <div className="py-2 text-xs text-low text-center">
                Live usage unavailable
              </div>
            )}

            {/* All-time stats from stats-cache */}
            {stats && (
              <>
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
              </>
            )}

            {stats && stats.models.length > 0 && (
              <>
                <div className="mt-1.5 h-px bg-border" />
                <SectionLabel>Output tokens (all time)</SectionLabel>
                {stats.models.map((m) => (
                  <div
                    key={m.name}
                    className="py-0.5 text-xs flex items-center justify-between"
                  >
                    <span className="text-low truncate mr-2">{m.name}</span>
                    <span className="font-medium tabular-nums text-normal whitespace-nowrap">
                      {formatNumber(m.outputTokens)}
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
