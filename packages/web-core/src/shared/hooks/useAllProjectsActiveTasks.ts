import { useState, useCallback } from 'react';
import { useProjects } from './useProjects';
import type { Project, TaskWithAttemptStatus } from 'shared/types';
import { handleApiResponse } from '@/lib/api';

export interface ProjectActiveTasks {
  project: Project;
  inReview: TaskWithAttemptStatus[];
  inProgress: TaskWithAttemptStatus[];
}

export function useAllProjectsActiveTasks() {
  const { projects } = useProjects();
  const [data, setData] = useState<ProjectActiveTasks[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  const refresh = useCallback(async () => {
    if (projects.length === 0) return;
    setIsLoading(true);
    try {
      const results = await Promise.all(
        projects.map(async (project) => {
          const response = await fetch(
            `/api/tasks?project_id=${encodeURIComponent(project.id)}`
          );
          const tasks =
            await handleApiResponse<TaskWithAttemptStatus[]>(response);
          return {
            project,
            inReview: tasks.filter((t) => t.status === 'inreview'),
            inProgress: tasks.filter((t) => t.status === 'inprogress'),
          };
        })
      );
      setData(
        results.filter((r) => r.inReview.length > 0 || r.inProgress.length > 0)
      );
    } catch (err) {
      console.error('Failed to fetch active tasks:', err);
    } finally {
      setIsLoading(false);
    }
  }, [projects]);

  const totalCount = data.reduce(
    (sum, p) => sum + p.inReview.length + p.inProgress.length,
    0
  );

  return { data, isLoading, refresh, totalCount };
}
