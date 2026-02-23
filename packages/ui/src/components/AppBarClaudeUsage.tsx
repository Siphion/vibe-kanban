import { GaugeIcon, SpinnerIcon } from '@phosphor-icons/react';
import { useMemo, useState } from 'react';
import { cn } from '@/lib/utils';
import { Popover, PopoverTrigger, PopoverContent } from '../primitives/Popover';
import { Tooltip } from '../primitives/Tooltip';
import { useClaudeUsage, type ClaudeStatsCache } from '@/hooks/useClaudeUsage';
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
    const weekToolCalls = weekActivity.reduce((s, d) => s + d.toolCallCount, 0);
    const weekOutputTokens = weekTokens.reduce(
      (s, d) => s + Object.values(d.tokensByModel).reduce((a, b) => a + b, 0),
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

function ProgressBar({ pct }: { pct: number }) {
  const clamped = Math.min(pct, 100);
  const color =
    clamped >= 90 ? 'bg-red-500' : clamped >= 70 ? 'bg-amber-500' : 'bg-brand';
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
}: {
  label: string;
  value: string | number;
  pct?: number;
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
  const { data, isLoading, refresh } = useClaudeUsage();
  const stats = useDerivedStats(data);
  const [planId, setPlanId] = useState(getSelectedPlanId);
  const plan = getPlanById(planId) ?? CLAUDE_PLANS[1];

  const handlePlanChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const id = e.target.value;
    setPlanId(id);
    setSelectedPlanId(id);
  };

  const pctToday = stats
    ? (stats.today.messages / plan.messagesPerDay) * 100
    : 0;
  const pctWeek = stats
    ? (stats.week.messages / (plan.messagesPerDay * 7)) * 100
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
        ) : !stats ? (
          <div className="py-4 text-sm text-low text-center">
            No usage data available
          </div>
        ) : (
          <div>
            <SectionLabel>Today</SectionLabel>
            <StatRow
              label="Messages"
              value={`${stats.today.messages.toLocaleString()} / ${plan.messagesPerDay.toLocaleString()}`}
              pct={pctToday}
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
              value={`${stats.week.messages.toLocaleString()} / ${(plan.messagesPerDay * 7).toLocaleString()}`}
              pct={pctWeek}
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
