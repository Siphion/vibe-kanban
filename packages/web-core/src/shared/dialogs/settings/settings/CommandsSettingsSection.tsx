import { useState, useCallback } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { PlusIcon, PencilSimpleIcon, TrashIcon } from '@phosphor-icons/react';
import {
  customCommandsApi,
  type CustomCommand,
  type CreateCustomCommand,
} from '@/shared/lib/api';
import {
  SettingsCard,
  SettingsField,
  SettingsInput,
  SettingsSelect,
  SettingsTextarea,
} from './SettingsComponents';

function CommandForm({
  initial,
  onSave,
  onCancel,
  saving,
}: {
  initial?: CustomCommand;
  onSave: (data: CreateCustomCommand) => void;
  onCancel: () => void;
  saving: boolean;
}) {
  const [name, setName] = useState(initial?.name ?? '');
  const [script, setScript] = useState(initial?.script ?? '');
  const [description, setDescription] = useState(initial?.description ?? '');
  const [mode, setMode] = useState(initial?.mode ?? 'background');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !script.trim()) return;
    onSave({
      name: name.trim(),
      script: script.trim(),
      description: description.trim() || undefined,
      mode,
    });
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-3">
      <SettingsField label="Name">
        <SettingsInput
          value={name}
          onChange={setName}
          placeholder="e.g., expose"
        />
      </SettingsField>
      <SettingsField label="Description (optional)">
        <SettingsInput
          value={description}
          onChange={setDescription}
          placeholder="Short description"
        />
      </SettingsField>
      <SettingsField label="Mode">
        <SettingsSelect
          value={mode}
          onChange={setMode}
          options={[
            { value: 'background', label: 'Background (start/stop)' },
            { value: 'oneshot', label: 'One-shot (run once)' },
          ]}
        />
      </SettingsField>
      <SettingsField label="Script">
        <SettingsTextarea
          value={script}
          onChange={setScript}
          placeholder="#!/bin/bash"
          rows={4}
          monospace
        />
      </SettingsField>
      <div className="flex gap-2 justify-end">
        <button
          type="button"
          onClick={onCancel}
          className="px-3 py-1.5 text-sm rounded-sm border border-border text-normal hover:bg-secondary"
        >
          Cancel
        </button>
        <button
          type="submit"
          disabled={saving || !name.trim() || !script.trim()}
          className="px-3 py-1.5 text-sm rounded-sm bg-brand text-white hover:bg-brand/90 disabled:opacity-50"
        >
          {saving ? 'Saving...' : initial ? 'Update' : 'Create'}
        </button>
      </div>
    </form>
  );
}

export function CommandsSettingsSectionContent() {
  const queryClient = useQueryClient();
  const [editingId, setEditingId] = useState<string | null>(null);
  const [showCreate, setShowCreate] = useState(false);

  const { data: commands = [], isLoading } = useQuery({
    queryKey: ['custom-commands'],
    queryFn: customCommandsApi.list,
  });

  const createMutation = useMutation({
    mutationFn: customCommandsApi.create,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['custom-commands'] });
      setShowCreate(false);
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: CreateCustomCommand }) =>
      customCommandsApi.update(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['custom-commands'] });
      setEditingId(null);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: customCommandsApi.delete,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['custom-commands'] });
    },
  });

  const handleDelete = useCallback(
    (id: string, name: string) => {
      if (confirm(`Delete command "${name}"?`)) {
        deleteMutation.mutate(id);
      }
    },
    [deleteMutation]
  );

  if (isLoading) {
    return (
      <SettingsCard title="Custom Commands" description="Loading...">
        <div />
      </SettingsCard>
    );
  }

  return (
    <SettingsCard
      title="Custom Commands"
      description='Define commands triggered via the Teams bot. Background commands use "<name> start/stop". One-shot commands run with "<name>" and return output.'
      headerAction={
        !showCreate && (
          <button
            onClick={() => setShowCreate(true)}
            className="flex items-center gap-1 px-2 py-1 text-xs rounded-sm bg-brand text-white hover:bg-brand/90"
          >
            <PlusIcon className="size-3.5" weight="bold" />
            Add Command
          </button>
        )
      }
    >
      {showCreate && (
        <div className="mb-4 p-3 border border-border rounded-sm bg-secondary/30">
          <CommandForm
            onSave={(data) => createMutation.mutate(data)}
            onCancel={() => setShowCreate(false)}
            saving={createMutation.isPending}
          />
        </div>
      )}

      {commands.length === 0 && !showCreate && (
        <p className="text-sm text-low py-4">
          No custom commands yet. Click &quot;Add Command&quot; to create one.
        </p>
      )}

      <div className="space-y-2">
        {commands.map((cmd) =>
          editingId === cmd.id ? (
            <div
              key={cmd.id}
              className="p-3 border border-border rounded-sm bg-secondary/30"
            >
              <CommandForm
                initial={cmd}
                onSave={(data) => updateMutation.mutate({ id: cmd.id, data })}
                onCancel={() => setEditingId(null)}
                saving={updateMutation.isPending}
              />
            </div>
          ) : (
            <div
              key={cmd.id}
              className="flex items-start justify-between gap-3 p-3 border border-border rounded-sm"
            >
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-high font-mono">
                    {cmd.name}
                  </span>
                  <span className="text-[10px] px-1 py-0.5 rounded bg-secondary text-low">
                    {cmd.mode === 'oneshot' ? 'one-shot' : 'background'}
                  </span>
                  {cmd.description && (
                    <span className="text-xs text-low truncate">
                      — {cmd.description}
                    </span>
                  )}
                </div>
                <pre className="mt-1 text-xs text-low font-mono truncate max-w-full">
                  {cmd.script.length > 80
                    ? cmd.script.slice(0, 80) + '...'
                    : cmd.script}
                </pre>
              </div>
              <div className="flex items-center gap-1 shrink-0">
                <button
                  onClick={() => setEditingId(cmd.id)}
                  className="p-1 rounded-sm hover:bg-secondary text-low hover:text-normal"
                  title="Edit"
                >
                  <PencilSimpleIcon className="size-3.5" weight="bold" />
                </button>
                <button
                  onClick={() => handleDelete(cmd.id, cmd.name)}
                  className="p-1 rounded-sm hover:bg-danger/10 text-low hover:text-danger"
                  title="Delete"
                >
                  <TrashIcon className="size-3.5" weight="bold" />
                </button>
              </div>
            </div>
          )
        )}
      </div>
    </SettingsCard>
  );
}
