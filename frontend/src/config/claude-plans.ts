/**
 * Claude plan limits configuration.
 *
 * These values are community-reported estimates and may change as Anthropic
 * updates their plans. Edit this file to keep limits up to date.
 *
 * Sources:
 * - https://support.claude.com/en/articles/11145838
 * - https://claudelog.com/claude-code-limits/
 *
 * Last updated: 2026-02-23
 */

export interface ClaudePlanLimits {
  id: string;
  name: string;
  /** Messages per 5-hour window (approximate) */
  messagesPer5h: number;
  /** Estimated daily message capacity (assumes ~3 active windows) */
  messagesPerDay: number;
  /** Price in USD/month */
  pricePerMonth: number;
}

export const CLAUDE_PLANS: ClaudePlanLimits[] = [
  {
    id: 'pro',
    name: 'Pro ($20/mo)',
    messagesPer5h: 45,
    messagesPerDay: 135,
    pricePerMonth: 20,
  },
  {
    id: 'max-5x',
    name: 'Max 5x ($100/mo)',
    messagesPer5h: 225,
    messagesPerDay: 675,
    pricePerMonth: 100,
  },
  {
    id: 'max-20x',
    name: 'Max 20x ($200/mo)',
    messagesPer5h: 900,
    messagesPerDay: 2700,
    pricePerMonth: 200,
  },
];

const STORAGE_KEY = 'claude-plan-id';

export function getSelectedPlanId(): string {
  try {
    return localStorage.getItem(STORAGE_KEY) ?? 'max-5x';
  } catch {
    return 'max-5x';
  }
}

export function setSelectedPlanId(id: string): void {
  try {
    localStorage.setItem(STORAGE_KEY, id);
  } catch {
    // ignore
  }
}

export function getPlanById(id: string): ClaudePlanLimits | undefined {
  return CLAUDE_PLANS.find((p) => p.id === id);
}
