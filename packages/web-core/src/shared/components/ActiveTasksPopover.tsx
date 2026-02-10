import { Activity, Loader2 } from 'lucide-react';
import { Link } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
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
  useAllProjectsActiveTasks,
  type ProjectActiveTasks,
} from '@/hooks/useAllProjectsActiveTasks';
import type { TaskWithAttemptStatus } from 'shared/types';

function StatusSection({
  label,
  color,
  entries,
}: {
  label: string;
  color: string;
  entries: { project: ProjectActiveTasks['project']; tasks: TaskWithAttemptStatus[] }[];
}) {
  if (entries.length === 0) return null;

  const count = entries.reduce((sum, e) => sum + e.tasks.length, 0);

  return (
    <>
      <DropdownMenuLabel
        className="text-xs font-semibold uppercase tracking-wide"
        style={{ color }}
      >
        {label} ({count})
      </DropdownMenuLabel>
      {entries.map((entry) => (
        <div key={entry.project.id}>
          <div className="px-2 py-1 text-xs text-muted-foreground truncate">
            {entry.project.name}
          </div>
          {entry.tasks.map((task) => (
            <DropdownMenuItem key={task.id} asChild>
              <Link
                to={`/local-projects/${entry.project.id}/tasks?taskId=${task.id}`}
                className="pl-4 truncate"
                title={task.title}
              >
                {task.title}
              </Link>
            </DropdownMenuItem>
          ))}
        </div>
      ))}
    </>
  );
}

export function ActiveTasksPopover() {
  const { data, isLoading, refresh, totalCount } = useAllProjectsActiveTasks();

  const reviewEntries = data
    .filter((d) => d.inReview.length > 0)
    .map((d) => ({ project: d.project, tasks: d.inReview }));

  const progressEntries = data
    .filter((d) => d.inProgress.length > 0)
    .map((d) => ({ project: d.project, tasks: d.inProgress }));

  return (
    <DropdownMenu onOpenChange={(open) => open && refresh()}>
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="hidden sm:inline-flex h-6 w-6 ml-2 relative"
                aria-label="Active tasks across all projects"
              >
                <Activity className="h-3.5 w-3.5" />
                {totalCount > 0 && (
                  <span className="absolute -top-1 -right-1 bg-primary text-primary-foreground text-[10px] font-medium rounded-full min-w-[16px] h-4 flex items-center justify-center px-1">
                    {totalCount}
                  </span>
                )}
              </Button>
            </DropdownMenuTrigger>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            Active tasks (In Progress &amp; In Review)
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>

      <DropdownMenuContent align="start" className="w-72">
        <DropdownMenuLabel className="text-sm font-semibold">
          Active Tasks
        </DropdownMenuLabel>
        <DropdownMenuSeparator />

        {isLoading ? (
          <div className="flex items-center justify-center py-4">
            <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
          </div>
        ) : data.length === 0 ? (
          <div className="px-2 py-4 text-sm text-muted-foreground text-center">
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
              <DropdownMenuSeparator />
            )}
            <StatusSection
              label="In Progress"
              color="hsl(var(--info))"
              entries={progressEntries}
            />
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
