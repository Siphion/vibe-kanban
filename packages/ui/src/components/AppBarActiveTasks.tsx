import { PulseIcon, SpinnerIcon } from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';
import { cn } from '@/lib/utils';
import { Popover, PopoverTrigger, PopoverContent } from '../primitives/Popover';
import { Tooltip } from '../primitives/Tooltip';
import {
  useAllProjectsActiveTasks,
  type ProjectActiveTasks,
} from '@/hooks/useAllProjectsActiveTasks';
import type { TaskWithAttemptStatus } from '@/hooks/useAllProjectsActiveTasks';

function StatusSection({
  label,
  color,
  entries,
}: {
  label: string;
  color: string;
  entries: {
    project: ProjectActiveTasks['project'];
    tasks: TaskWithAttemptStatus[];
  }[];
}) {
  if (entries.length === 0) return null;

  const count = entries.reduce((sum, e) => sum + e.tasks.length, 0);

  return (
    <div className="mt-2">
      <div
        className="text-[10px] font-semibold uppercase tracking-wide mb-0.5"
        style={{ color }}
      >
        {label} ({count})
      </div>
      {entries.map((entry) => (
        <div key={entry.project.id}>
          <div className="py-1 text-xs text-low truncate">
            {entry.project.name}
          </div>
          {entry.tasks.map((task) => (
            <Link
              key={task.id}
              to="/projects/$projectId/issues/$issueId"
              params={{ projectId: entry.project.id, issueId: task.id }}
              className={cn(
                'block pl-2 py-1 text-xs text-normal truncate rounded-sm',
                'hover:bg-brand/10 transition-colors'
              )}
              title={task.title}
            >
              {task.title}
            </Link>
          ))}
        </div>
      ))}
    </div>
  );
}

export function AppBarActiveTasks() {
  const { data, isLoading, refresh, totalCount } = useAllProjectsActiveTasks();

  const reviewEntries = data
    .filter((d) => d.inReview.length > 0)
    .map((d) => ({ project: d.project, tasks: d.inReview }));

  const progressEntries = data
    .filter((d) => d.inProgress.length > 0)
    .map((d) => ({ project: d.project, tasks: d.inProgress }));

  return (
    <Popover onOpenChange={(open: boolean) => open && refresh()}>
      <Tooltip content="Active tasks (In Progress & In Review)" side="right">
        <PopoverTrigger asChild>
          <button
            type="button"
            className={cn(
              'relative flex items-center justify-center w-10 h-10 rounded-lg',
              'transition-colors cursor-pointer',
              'focus:outline-none focus-visible:ring-2 focus-visible:ring-brand',
              'bg-primary text-normal hover:bg-brand/10'
            )}
            aria-label="Active tasks across all projects"
          >
            <PulseIcon className="size-icon-base" weight="bold" />
            {totalCount > 0 && (
              <span
                className={cn(
                  'absolute -top-1 -right-1',
                  'bg-brand text-on-brand text-[10px] font-medium',
                  'rounded-full min-w-[16px] h-4',
                  'flex items-center justify-center px-1'
                )}
              >
                {totalCount}
              </span>
            )}
          </button>
        </PopoverTrigger>
      </Tooltip>

      <PopoverContent side="right" sideOffset={8} className="w-72 p-3">
        <p className="text-sm font-semibold text-high">Active Tasks</p>

        <div className="mt-2 h-px bg-border" />

        {isLoading ? (
          <div className="flex items-center justify-center py-4">
            <SpinnerIcon className="size-4 animate-spin text-low" />
          </div>
        ) : data.length === 0 ? (
          <div className="py-4 text-sm text-low text-center">
            No active tasks
          </div>
        ) : (
          <>
            <StatusSection
              label="In Review"
              color="hsl(var(--warning))"
              entries={reviewEntries}
            />
            {reviewEntries.length > 0 && progressEntries.length > 0 && (
              <div className="mt-1.5 h-px bg-border" />
            )}
            <StatusSection
              label="In Progress"
              color="hsl(var(--info))"
              entries={progressEntries}
            />
          </>
        )}
      </PopoverContent>
    </Popover>
  );
}
