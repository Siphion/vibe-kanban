import { GaugeIcon, SpinnerIcon } from '@phosphor-icons/react';
import { useMemo, useState } from 'react';
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
      week: {
        sessions: weekSessions,
      },
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
        <div className="flex items-center justify-between">
          <p className="text-sm font-semibold text-high">Claude Code Usage</p>
        </div>

        {/* Plan selector */}
        <select
          value={planId}
          onChange={handlePlanChange}
          className={cn(
            'mt-1.5 w-full rounded-md px-2 py-1 text-xs',
            'bg-surface-secondary text-normal border border-border',
            'focus:outline-none focus:ring-1 focus:ring-brand',
            'cursor-pointer'
          )}
        >
          {CLAUDE_PLANS.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>

        <div className="mt-2 h-px bg-border" />

        {isLoading ? (
          <div className="flex items-center justify-center py-4">
            <SpinnerIcon className="size-4 animate-spin text-low" />
          </div>
        ) : !liveData ? (
          <div className="py-4 text-sm text-low text-center">
            No usage data available
          </div>
        ) : (
          <div>
            {/* 5-hour rolling window */}
            <SectionLabel>5h window</SectionLabel>
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

            <div className="mt-1.5 h-px bg-border" />

            {/* Today */}
            <SectionLabel>Today</SectionLabel>
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

            <div className="mt-1.5 h-px bg-border" />

            {/* Week */}
            <SectionLabel>Last 7 days</SectionLabel>
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
                <div className="mt-1.5 h-px bg-border" />
                <SectionLabel>Sessions in window</SectionLabel>
                {liveData.sessions.map((s) => {
                  const project = s.project.split('/').pop() ?? s.project;
                  return (
                    <div
                      key={s.sessionId}
                      className="py-0.5 text-xs flex items-center justify-between"
                    >
                      <span className="text-low truncate mr-2" title={s.project}>
                        {project}
                      </span>
                      <span className="font-medium tabular-nums text-normal whitespace-nowrap">
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
                <div className="mt-1.5 h-px bg-border" />
                <SectionLabel>By model (all time)</SectionLabel>
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

            {/* Limits note */}
            <div className="mt-2 pt-1.5 border-t border-border">
              <p className="text-[10px] text-low leading-tight">
                Limits are estimates (~{plan.messagesPer5h} msg/5h window, ~3
                windows/day). Edit{' '}
                <span className="font-mono">config/claude-plans.ts</span> to
                update.
              </p>
            </div>
          </div>
        )}
      </PopoverContent>
    </Popover>
  );
}
