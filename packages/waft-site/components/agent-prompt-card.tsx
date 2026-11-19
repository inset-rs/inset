'use client';

import { useState } from 'react';
import { Check, Copy, Sparkles } from 'lucide-react';
import { buttonVariants } from 'fumadocs-ui/components/ui/button';
import { useCopyButton } from 'fumadocs-ui/utils/use-copy-button';
import { agentPrompt } from '@/lib/shared';

const starters = [
  { label: 'Markdown notes', idea: 'Markdown notes app with WinUI and Cupertino views' },
  { label: 'REST client', idea: 'REST API inspector with request history' },
  { label: 'Window switcher', idea: 'fast keyboard-driven window switcher' },
  { label: 'Habit tracker', idea: 'mobile and desktop habit tracker' },
] as const;

/**
 * Inline prompt card: lets the reader pick an optional starter idea, previews the exact prompt
 * pointing at `agents.md`, and copies it in one click.
 */
export function AgentPromptCard({ origin }: { origin: string }) {
  const [selected, setSelected] = useState<string | null>(null);
  const prompt = agentPrompt(selected ?? undefined, origin);
  const [copied, copy] = useCopyButton(() => navigator.clipboard.writeText(prompt));

  return (
    <div className="rounded-3xl border bg-fd-card p-6 sm:p-8">
      <div className="flex flex-wrap items-center gap-2">
        <span className="mr-1 inline-flex items-center gap-1.5 text-sm font-medium text-fd-muted-foreground">
          <Sparkles aria-hidden className="size-4" />
          Try asking for:
        </span>
        {starters.map((starter) => {
          const active = selected === starter.idea;
          return (
            <button
              key={starter.label}
              type="button"
              aria-pressed={active}
              onClick={() => setSelected(active ? null : starter.idea)}
              className={`rounded-full border px-3.5 py-1.5 text-sm font-medium transition-colors ${
                active
                  ? 'border-fd-foreground bg-fd-foreground text-fd-background'
                  : 'border-fd-border bg-fd-background text-fd-foreground hover:bg-fd-accent'
              }`}
            >
              {starter.label}
            </button>
          );
        })}
      </div>

      <div className="mt-5 flex flex-col gap-3 rounded-2xl border bg-fd-background p-4 sm:flex-row sm:items-center sm:justify-between sm:pl-5">
        <code className="min-w-0 flex-1 text-sm leading-relaxed break-words sm:text-base">{prompt}</code>
        <button
          type="button"
          onClick={copy}
          className={buttonVariants({
            color: 'primary',
            className: 'shrink-0 gap-2 self-start px-4 py-2.5 text-sm sm:self-center',
          })}
        >
          {copied ? <Check aria-hidden className="size-4" /> : <Copy aria-hidden className="size-4" />}
          {copied ? 'Copied' : 'Copy prompt'}
        </button>
      </div>
    </div>
  );
}
