'use client';

import { useState } from 'react';
import { Monitor, Smartphone, Layers, Check, Copy, ExternalLink, Sparkles } from 'lucide-react';
import { galleryUrls } from '@/lib/shared';

type KitId = 'winui' | 'cupertino' | 'material';

interface KitSpec {
  id: KitId;
  label: string;
  crateName: string;
  targetBadge: string;
  galleryUrl?: string;
  galleryLabel?: string;
  importLine: string;
  rootWidget: string;
  codeLines: readonly { num: number; indent?: number; tokens: readonly { text: string; tone?: 'kw' | 'ty' | 'fn' | 'str' | 'mac' | 'dim' }[] }[];
}

const KITS: readonly KitSpec[] = [
  {
    id: 'winui',
    label: 'Fluent / WinUI',
    crateName: 'waft-winui',
    targetBadge: 'macOS, Windows, Linux & Web',
    galleryUrl: galleryUrls.winui,
    galleryLabel: 'Live WinUI Gallery',
    importLine: 'use waft_winui::{Button, FluentApp, NavigationView, TextBox, ToggleSwitch};',
    rootWidget: 'FluentApp::new().title("Studio").home(StudioView)',
    codeLines: [
      { num: 1, tokens: [{ text: '#![forbid(unsafe_code)]', tone: 'mac' }] },
      { num: 2, tokens: [{ text: 'use ', tone: 'kw' }, { text: 'waft::{App, BuildContext, IntoWidget, StatelessWidget, WidgetRef, run_app};' }] },
      { num: 3, tokens: [{ text: 'use ', tone: 'kw' }, { text: 'waft_winui', tone: 'ty' }, { text: '::{Button, FluentApp, NavigationView, ToggleSwitch};' }] },
      { num: 4, tokens: [] },
      { num: 5, tokens: [{ text: '#[waft::main]', tone: 'mac' }] },
      { num: 6, tokens: [{ text: 'fn ', tone: 'kw' }, { text: 'main', tone: 'fn' }, { text: '(app: &', tone: 'dim' }, { text: 'mut ', tone: 'kw' }, { text: 'App', tone: 'ty' }, { text: ') {' }] },
      { num: 7, indent: 1, tokens: [{ text: 'run_app', tone: 'fn' }, { text: '(app, ' }, { text: 'FluentApp', tone: 'ty' }, { text: '::new().title(' }, { text: '"Atlas Studio"', tone: 'str' }, { text: ').home(Studio).into_widget());' }] },
      { num: 8, tokens: [{ text: '}' }] },
      { num: 9, tokens: [] },
      { num: 10, tokens: [{ text: 'impl ', tone: 'kw' }, { text: 'StatelessWidget ', tone: 'ty' }, { text: 'for ', tone: 'kw' }, { text: 'Studio ', tone: 'ty' }, { text: '{' }] },
      { num: 11, indent: 1, tokens: [{ text: 'fn ', tone: 'kw' }, { text: 'build', tone: 'fn' }, { text: '(&self, _app: &', tone: 'dim' }, { text: 'mut ', tone: 'kw' }, { text: 'App', tone: 'ty' }, { text: ', _cx: ' }, { text: 'BuildContext', tone: 'ty' }, { text: ') -> ' }, { text: 'WidgetRef ', tone: 'ty' }, { text: '{' }] },
      { num: 12, indent: 2, tokens: [{ text: 'NavigationView', tone: 'ty' }, { text: '::new().pane_title(' }, { text: '"Workspace"', tone: 'str' }, { text: ')' }] },
      { num: 13, indent: 3, tokens: [{ text: '.item(' }, { text: '"Audio Engine"', tone: 'str' }, { text: ', AudioPanel::new())' }] },
      { num: 14, indent: 3, tokens: [{ text: '.item(' }, { text: '"Shortcuts"', tone: 'str' }, { text: ', ShortcutTable::new())' }] },
      { num: 15, indent: 3, tokens: [{ text: '.footer(' }, { text: 'Button', tone: 'ty' }, { text: '::accent(' }, { text: '"Export Build"', tone: 'str' }, { text: '))' }] },
      { num: 15, indent: 3, tokens: [{ text: '.into_widget()' }] },
      { num: 16, indent: 1, tokens: [{ text: '}' }] },
      { num: 17, tokens: [{ text: '}' }] },
    ],
  },
  {
    id: 'cupertino',
    label: 'Cupertino',
    crateName: 'waft-cupertino',
    targetBadge: 'iOS, iPadOS & Android',
    galleryUrl: galleryUrls.cupertino,
    galleryLabel: 'Live Cupertino Gallery',
    importLine: 'use waft_cupertino::{CupertinoApp, CupertinoListSection, CupertinoSliverNavigationBar, CupertinoSwitch};',
    rootWidget: 'CupertinoApp::new().home(MobileStudioView)',
    codeLines: [
      { num: 1, tokens: [{ text: '#![forbid(unsafe_code)]', tone: 'mac' }] },
      { num: 2, tokens: [{ text: 'use ', tone: 'kw' }, { text: 'waft::{App, BuildContext, CustomScrollView, IntoWidget, StatelessWidget};' }] },
      { num: 3, tokens: [{ text: 'use ', tone: 'kw' }, { text: 'waft_cupertino', tone: 'ty' }, { text: '::{CupertinoApp, CupertinoSliverNavigationBar};' }] },
      { num: 4, tokens: [] },
      { num: 5, tokens: [{ text: '#[waft::main]', tone: 'mac' }] },
      { num: 6, tokens: [{ text: 'fn ', tone: 'kw' }, { text: 'main', tone: 'fn' }, { text: '(app: &', tone: 'dim' }, { text: 'mut ', tone: 'kw' }, { text: 'App', tone: 'ty' }, { text: ') {' }] },
      { num: 7, indent: 1, tokens: [{ text: 'run_app', tone: 'fn' }, { text: '(app, ' }, { text: 'CupertinoApp', tone: 'ty' }, { text: '::new().home(MobileStudio).into_widget());' }] },
      { num: 8, tokens: [{ text: '}' }] },
      { num: 9, tokens: [] },
      { num: 10, tokens: [{ text: 'impl ', tone: 'kw' }, { text: 'StatelessWidget ', tone: 'ty' }, { text: 'for ', tone: 'kw' }, { text: 'MobileStudio ', tone: 'ty' }, { text: '{' }] },
      { num: 11, indent: 1, tokens: [{ text: 'fn ', tone: 'kw' }, { text: 'build', tone: 'fn' }, { text: '(&self, _app: &', tone: 'dim' }, { text: 'mut ', tone: 'kw' }, { text: 'App', tone: 'ty' }, { text: ', _cx: ' }, { text: 'BuildContext', tone: 'ty' }, { text: ') -> ' }, { text: 'WidgetRef ', tone: 'ty' }, { text: '{' }] },
      { num: 12, indent: 2, tokens: [{ text: 'CustomScrollView', tone: 'ty' }, { text: '::new()' }] },
      { num: 13, indent: 3, tokens: [{ text: '.sliver(' }, { text: 'CupertinoSliverNavigationBar', tone: 'ty' }, { text: '::new(' }, { text: '"Atlas Studio"', tone: 'str' }, { text: '))' }] },
      { num: 14, indent: 3, tokens: [{ text: '.sliver(' }, { text: 'CupertinoSwitch', tone: 'ty' }, { text: '::new(true))' }] },
            { num: 16, indent: 3, tokens: [{ text: '.into_widget()' }] },
      { num: 17, indent: 1, tokens: [{ text: '}' }] },
      { num: 18, tokens: [{ text: '}' }] },
    ],
  },
  {
    id: 'material',
    label: 'Material 3',
    crateName: 'waft-material',
    targetBadge: 'Android, Desktop & Web',
    importLine: 'use waft_material::{AppBar, FilledButton, MaterialApp, NavigationBar, Scaffold, Switch};',
    rootWidget: 'MaterialApp::new().home(MaterialStudioView)',
    codeLines: [
      { num: 1, tokens: [{ text: '#![forbid(unsafe_code)]', tone: 'mac' }] },
      { num: 2, tokens: [{ text: 'use ', tone: 'kw' }, { text: 'waft::{App, BuildContext, IntoWidget, StatelessWidget, WidgetRef, run_app};' }] },
      { num: 3, tokens: [{ text: 'use ', tone: 'kw' }, { text: 'waft_material', tone: 'ty' }, { text: '::{AppBar, Card, FilledButton, MaterialApp, Scaffold, Switch};' }] },
      { num: 4, tokens: [] },
      { num: 5, tokens: [{ text: '#[waft::main]', tone: 'mac' }] },
      { num: 6, tokens: [{ text: 'fn ', tone: 'kw' }, { text: 'main', tone: 'fn' }, { text: '(app: &', tone: 'dim' }, { text: 'mut ', tone: 'kw' }, { text: 'App', tone: 'ty' }, { text: ') {' }] },
      { num: 7, indent: 1, tokens: [{ text: 'run_app', tone: 'fn' }, { text: '(app, ' }, { text: 'MaterialApp', tone: 'ty' }, { text: '::new().home(MaterialStudio).into_widget());' }] },
      { num: 8, tokens: [{ text: '}' }] },
      { num: 9, tokens: [] },
      { num: 10, tokens: [{ text: 'impl ', tone: 'kw' }, { text: 'StatelessWidget ', tone: 'ty' }, { text: 'for ', tone: 'kw' }, { text: 'MaterialStudio ', tone: 'ty' }, { text: '{' }] },
      { num: 11, indent: 1, tokens: [{ text: 'fn ', tone: 'kw' }, { text: 'build', tone: 'fn' }, { text: '(&self, _app: &', tone: 'dim' }, { text: 'mut ', tone: 'kw' }, { text: 'App', tone: 'ty' }, { text: ', _cx: ' }, { text: 'BuildContext', tone: 'ty' }, { text: ') -> ' }, { text: 'WidgetRef ', tone: 'ty' }, { text: '{' }] },
      { num: 12, indent: 2, tokens: [{ text: 'Scaffold', tone: 'ty' }, { text: '::new().app_bar(' }, { text: 'AppBar', tone: 'ty' }, { text: '::new(' }, { text: '"Atlas Studio"', tone: 'str' }, { text: '))' }] },
      { num: 13, indent: 3, tokens: [{ text: '.body(' }, { text: 'Card', tone: 'ty' }, { text: '::elevated().child(' }, { text: 'Switch', tone: 'ty' }, { text: '::new(true)))' }] },
      { num: 14, indent: 3, tokens: [{ text: '.floating_action(' }, { text: 'FilledButton', tone: 'ty' }, { text: '::new(' }, { text: '"Export Build"', tone: 'str' }, { text: '))' }] },
      { num: 15, indent: 3, tokens: [{ text: '.into_widget()' }] },
      { num: 16, indent: 1, tokens: [{ text: '}' }] },
      { num: 17, tokens: [{ text: '}' }] },
    ],
  },
];

const TONE_COLOR: Record<string, string> = {
  kw: '#c84b11',
  ty: '#005fb8',
  fn: '#7c3aed',
  str: '#15803d',
  mac: '#b45309',
  dim: 'rgba(20, 20, 19, 0.5)',
};

export function HeroStudio() {
  const [activeKit, setActiveKit] = useState<KitId>('winui');
  const [lowLatency, setLowLatency] = useState(true);
  const [spatialDsp, setSpatialDsp] = useState(true);
  const [midiSync, setMidiSync] = useState(false);
  const [activeNav, setActiveNav] = useState<'audio' | 'shortcuts' | 'targets'>('audio');
  const [copiedCmd, setCopiedCmd] = useState(false);

  const spec = KITS.find((k) => k.id === activeKit) ?? KITS[0];
  const cliCommand = 'cargo install waft-cli && cargo waft new notes && cargo waft run';

  function copyCli() {
    navigator.clipboard?.writeText(cliCommand);
    setCopiedCmd(true);
    setTimeout(() => setCopiedCmd(false), 1800);
  }

  return (
    <div className="mx-auto w-full max-w-[1160px]">
      {/* Top control bar of the studio */}
      <div
        className="overflow-hidden rounded-[22px] border shadow-[0_24px_70px_-28px_rgba(20,20,19,0.16)]"
        style={{
          background: 'var(--site-card)',
          borderColor: 'var(--site-hairline-strong)',
        }}
      >
        <div
          className="flex flex-wrap items-center justify-between gap-3 border-b px-4 py-3 sm:px-6"
          style={{
            background: 'var(--site-surface)',
            borderColor: 'var(--site-hairline)',
          }}
        >
          {/* Left: Kit Switcher */}
          <div className="flex flex-wrap items-center gap-1.5">
            <span
              className="mr-2 font-mono text-[11px] font-semibold uppercase tracking-[0.08em]"
              style={{ color: 'var(--site-subtle)' }}
            >
              UI Kit
            </span>
            {KITS.map((k) => {
              const selected = k.id === activeKit;
              const Icon = k.id === 'winui' ? Monitor : k.id === 'cupertino' ? Smartphone : Layers;
              return (
                <button
                  key={k.id}
                  type="button"
                  onClick={() => setActiveKit(k.id)}
                  className="inline-flex items-center gap-2 rounded-lg px-3 py-1.5 text-[12.5px] font-medium transition"
                  style={{
                    background: selected ? 'var(--site-ink)' : 'transparent',
                    color: selected ? 'var(--site-bg)' : 'var(--site-muted)',
                    boxShadow: selected ? '0 2px 8px rgba(20,20,19,0.15)' : 'none',
                  }}
                >
                  <Icon className="size-3.5" />
                  <span>{k.label}</span>
                  <code
                    className="rounded px-1.5 py-0.5 font-mono text-[10.5px]"
                    style={{
                      background: selected ? 'rgba(255,255,255,0.14)' : 'var(--site-hairline)',
                      color: selected ? 'var(--site-bg)' : 'var(--site-muted)',
                    }}
                  >
                    {k.crateName}
                  </code>
                </button>
              );
            })}
          </div>

          {/* Right: Target badge & gallery link */}
          <div className="flex items-center gap-3 text-[12px]">
            <span
              className="hidden rounded-md border px-2.5 py-1 font-mono text-[11px] md:inline-block"
              style={{
                borderColor: 'var(--site-hairline)',
                background: 'var(--site-card)',
                color: 'var(--site-muted)',
              }}
            >
              {spec.targetBadge}
            </span>
            {spec.galleryUrl ? (
              <a
                href={spec.galleryUrl}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1 font-medium underline decoration-black/25 underline-offset-4 transition hover:opacity-80 dark:decoration-white/25"
                style={{ color: 'var(--site-ink)' }}
              >
                {spec.galleryLabel}
                <ExternalLink className="size-3" />
              </a>
            ) : null}
          </div>
        </div>

        {/* Main 2-column split: Rust Source on left, Live Interactive Kit surface on right */}
        <div className="grid grid-cols-1 lg:grid-cols-12">
          {/* Left 7 cols: Pure Rust Widget Code */}
          <div
            className="flex flex-col justify-between border-b p-5 sm:p-6 lg:col-span-7 lg:border-r lg:border-b-0"
            style={{
              borderColor: 'var(--site-hairline)',
              background: 'var(--site-card)',
            }}
          >
            <div>
              <div className="mb-3 flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-[12px] font-semibold" style={{ color: 'var(--site-ink)' }}>
                    src/lib.rs
                  </span>
                  <span
                    className="rounded px-2 py-0.5 font-mono text-[10.5px]"
                    style={{ background: 'var(--site-surface)', color: 'var(--site-muted)' }}
                  >
                    100% safe Rust
                  </span>
                </div>
                <span className="font-mono text-[11px]" style={{ color: 'var(--site-subtle)' }}>
                  Flutter widget model — valo + wgpu
                </span>
              </div>

              <div
                className="overflow-x-auto rounded-xl border p-4 font-mono text-[12px] leading-[20px]"
                style={{
                  background: '#fcfbf9',
                  borderColor: 'var(--site-hairline)',
                  color: '#1c1b18',
                }}
              >
                {spec.codeLines.map((line) => (
                  <div key={line.num} className="flex gap-3 whitespace-pre">
                    <span className="w-5 shrink-0 text-right select-none" style={{ color: 'rgba(20,20,19,0.28)' }}>
                      {line.num}
                    </span>
                    <span style={{ paddingLeft: `${(line.indent ?? 0) * 14}px` }}>
                      {line.tokens.map((tok, i) => (
                        <span
                          key={i}
                          style={{
                            color: tok.tone ? TONE_COLOR[tok.tone] : '#1c1b18',
                            fontWeight: tok.tone === 'kw' || tok.tone === 'ty' ? 560 : 450,
                          }}
                        >
                          {tok.text}
                        </span>
                      ))}
                    </span>
                  </div>
                ))}
              </div>
            </div>

            {/* Bottom CLI bar inside code column */}
            <div
              className="mt-4 flex items-center justify-between gap-3 rounded-xl border px-3.5 py-2.5"
              style={{
                background: 'var(--site-surface)',
                borderColor: 'var(--site-hairline)',
              }}
            >
              <div className="flex items-center gap-2 overflow-x-auto font-mono text-[12px]">
                <span style={{ color: 'var(--site-accent)' }}>$</span>
                <pre className="m-0 p-0 bg-transparent"><code style={{ color: 'var(--site-ink)' }}>{cliCommand}</code></pre>
              </div>
              <button
                type="button"
                onClick={copyCli}
                className="shrink-0 inline-flex items-center gap-1.5 rounded-md border px-2.5 py-1 text-[11.5px] font-medium transition hover:opacity-80"
                style={{
                  background: 'var(--site-card)',
                  borderColor: 'var(--site-hairline)',
                  color: 'var(--site-ink)',
                }}
              >
                {copiedCmd ? <Check className="size-3.5 text-emerald-600" /> : <Copy className="size-3.5" />}
                {copiedCmd ? 'Copied' : 'Copy command'}
              </button>
            </div>
          </div>

          {/* Right 5 cols: Interactive Rendered Kit Surface */}
          <div className="waft-blueprint-panel flex flex-col justify-between p-5 sm:p-6 lg:col-span-5">
            {activeKit === 'winui' ? (
              /* Desktop WinUI window preview */
              <div
                className="overflow-hidden rounded-xl border bg-white text-[#18181b] shadow-[0_14px_34px_-14px_rgba(0,0,0,0.16)]"
                style={{ borderColor: 'rgba(0,0,0,0.12)' }}
              >
                {/* Titlebar */}
                <div className="flex items-center justify-between border-b border-black/[0.07] bg-[#f4f4f6] px-3.5 py-2">
                  <div className="flex items-center gap-1.5">
                    <span className="size-2.5 rounded-full bg-[#ff5f57]" />
                    <span className="size-2.5 rounded-full bg-[#febc2e]" />
                    <span className="size-2.5 rounded-full bg-[#28c840]" />
                  </div>
                  <span className="text-[11.5px] font-medium text-zinc-600">Atlas Studio — waft-winui</span>
                  <span className="rounded bg-blue-600/10 px-1.5 py-0.5 font-mono text-[10px] font-semibold text-blue-700">
                    wgpu
                  </span>
                </div>

                {/* NavigationView body */}
                <div className="grid grid-cols-12">
                  {/* Left NavigationPane */}
                  <div className="col-span-4 border-r border-black/[0.06] bg-[#f9f9fb] p-2.5 text-[11.5px]">
                    <div className="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-wider text-zinc-400">
                      Workspace
                    </div>
                    {(
                      [
                        { id: 'audio', label: 'Audio Engine' },
                        { id: 'shortcuts', label: 'Shortcuts' },
                        { id: 'targets', label: 'Build Targets' },
                      ] as const
                    ).map((tab) => {
                      const active = activeNav === tab.id;
                      return (
                        <button
                          key={tab.id}
                          type="button"
                          onClick={() => setActiveNav(tab.id)}
                          className="mb-1 flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left font-medium transition"
                          style={{
                            background: active ? 'rgba(0, 95, 184, 0.1)' : 'transparent',
                            color: active ? '#005fb8' : '#3f3f46',
                          }}
                        >
                          <span
                            className="h-3.5 w-1 rounded-full"
                            style={{ background: active ? '#005fb8' : 'transparent' }}
                          />
                          {tab.label}
                        </button>
                      );
                    })}
                  </div>

                  {/* Right Content Pane */}
                  <div className="col-span-8 flex flex-col justify-between p-3.5">
                    <div>
                      <div className="flex items-center justify-between">
                        <span className="text-[13px] font-semibold text-zinc-900">
                          {activeNav === 'audio'
                            ? 'Real-time DSP Pipeline'
                            : activeNav === 'shortcuts'
                              ? 'Hardware Keymap'
                              : 'Native Packaging'}
                        </span>
                        <span className="rounded-full bg-emerald-500/12 px-2 py-0.5 font-mono text-[10px] font-medium text-emerald-700">
                          48 kHz — 1.2 ms
                        </span>
                      </div>

                      <div className="mt-3 space-y-2">
                        <label className="flex cursor-pointer items-center justify-between rounded-lg border border-black/[0.07] bg-zinc-50/70 px-2.5 py-2">
                          <div>
                            <div className="text-[11.5px] font-medium text-zinc-800">Low-Latency CoreAudio</div>
                            <div className="text-[10px] text-zinc-500">Exclusive buffer mode</div>
                          </div>
                          <button
                            type="button"
                            onClick={() => setLowLatency(!lowLatency)}
                            className="relative h-5 w-9 rounded-full transition"
                            style={{ background: lowLatency ? '#005fb8' : '#d4d4d8' }}
                          >
                            <span
                              className="absolute top-0.5 size-4 rounded-full bg-white shadow transition-all"
                              style={{ left: lowLatency ? '18px' : '2px' }}
                            />
                          </button>
                        </label>

                        <label className="flex cursor-pointer items-center justify-between rounded-lg border border-black/[0.07] bg-zinc-50/70 px-2.5 py-2">
                          <div>
                            <div className="text-[11.5px] font-medium text-zinc-800">Spatial Binaural Renderer</div>
                            <div className="text-[10px] text-zinc-500">GPU compute convolution</div>
                          </div>
                          <button
                            type="button"
                            onClick={() => setSpatialDsp(!spatialDsp)}
                            className="relative h-5 w-9 rounded-full transition"
                            style={{ background: spatialDsp ? '#005fb8' : '#d4d4d8' }}
                          >
                            <span
                              className="absolute top-0.5 size-4 rounded-full bg-white shadow transition-all"
                              style={{ left: spatialDsp ? '18px' : '2px' }}
                            />
                          </button>
                        </label>

                        <label className="flex cursor-pointer items-center justify-between rounded-lg border border-black/[0.07] bg-zinc-50/70 px-2.5 py-2">
                          <div>
                            <div className="text-[11.5px] font-medium text-zinc-800">External MIDI Clock</div>
                            <div className="text-[10px] text-zinc-500">USB & network RTP sync</div>
                          </div>
                          <button
                            type="button"
                            onClick={() => setMidiSync(!midiSync)}
                            className="relative h-5 w-9 rounded-full transition"
                            style={{ background: midiSync ? '#005fb8' : '#d4d4d8' }}
                          >
                            <span
                              className="absolute top-0.5 size-4 rounded-full bg-white shadow transition-all"
                              style={{ left: midiSync ? '18px' : '2px' }}
                            />
                          </button>
                        </label>
                      </div>
                    </div>

                    <div className="mt-3.5 flex items-center justify-between border-t border-black/[0.06] pt-2.5">
                      <span className="font-mono text-[10.5px] text-zinc-500">⌘E — Export master</span>
                      <button
                        type="button"
                        onClick={() => setMidiSync(!midiSync)}
                        className="rounded-md bg-[#005fb8] px-3 py-1 text-[11.5px] font-medium text-white shadow-sm transition hover:bg-[#004e98]"
                      >
                        Export Build
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            ) : activeKit === 'cupertino' ? (
              /* Mobile Cupertino phone preview */
              <div className="mx-auto w-full max-w-[300px] overflow-hidden rounded-[28px] border-[5px] border-[#1c1c1e] bg-[#f2f2f7] text-[#1c1c1e] shadow-[0_16px_36px_-14px_rgba(0,0,0,0.22)]">
                {/* Status bar & large title */}
                <div className="bg-white/90 px-4 pt-2.5 pb-3 backdrop-blur">
                  <div className="flex items-center justify-between text-[10.5px] font-semibold text-zinc-500">
                    <span>9:41</span>
                    <span className="font-mono text-[10px] text-[#007aff]">waft-cupertino</span>
                    <span>100%</span>
                  </div>
                  <div className="mt-2 flex items-baseline justify-between">
                    <span className="text-[19px] font-bold tracking-tight text-black">Atlas Studio</span>
                    <span className="text-[12px] font-medium text-[#007aff]">Done</span>
                  </div>
                </div>

                {/* Inset grouped list */}
                <div className="p-3.5">
                  <div className="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-wider text-zinc-500">
                    DSP & Haptics — Touch & IME Ready
                  </div>
                  <div className="divide-y divide-black/[0.07] overflow-hidden rounded-xl bg-white shadow-xs">
                    <div className="flex items-center justify-between px-3 py-2.5">
                      <div>
                        <div className="text-[12.5px] font-medium">Low-Latency Audio</div>
                        <div className="text-[10.5px] text-zinc-500">48 kHz AAudio / CoreAudio</div>
                      </div>
                      <button
                        type="button"
                        onClick={() => setLowLatency(!lowLatency)}
                        className="relative h-5 w-9 rounded-full transition"
                        style={{ background: lowLatency ? '#34c759' : '#e5e5ea' }}
                      >
                        <span
                          className="absolute top-0.5 size-4 rounded-full bg-white shadow transition-all"
                          style={{ left: lowLatency ? '18px' : '2px' }}
                        />
                      </button>
                    </div>

                    <div className="flex items-center justify-between px-3 py-2.5">
                      <div>
                        <div className="text-[12.5px] font-medium">Spatial Binaural</div>
                        <div className="text-[10.5px] text-zinc-500">Metal / Vulkan compute</div>
                      </div>
                      <button
                        type="button"
                        onClick={() => setSpatialDsp(!spatialDsp)}
                        className="relative h-5 w-9 rounded-full transition"
                        style={{ background: spatialDsp ? '#34c759' : '#e5e5ea' }}
                      >
                        <span
                          className="absolute top-0.5 size-4 rounded-full bg-white shadow transition-all"
                          style={{ left: spatialDsp ? '18px' : '2px' }}
                        />
                      </button>
                    </div>

                    <div className="flex items-center justify-between px-3 py-2.5">
                      <div>
                        <div className="text-[12.5px] font-medium">MIDI Clock Sync</div>
                        <div className="text-[10.5px] text-zinc-500">Sheet & gesture navigation</div>
                      </div>
                      <button
                        type="button"
                        onClick={() => setMidiSync(!midiSync)}
                        className="relative h-5 w-9 rounded-full transition"
                        style={{ background: midiSync ? '#34c759' : '#e5e5ea' }}
                      >
                        <span
                          className="absolute top-0.5 size-4 rounded-full bg-white shadow transition-all"
                          style={{ left: midiSync ? '18px' : '2px' }}
                        />
                      </button>
                    </div>
                  </div>

                  {/* Home indicator */}
                  <div className="mt-3 flex justify-center">
                    <div className="h-1 w-24 rounded-full bg-black/25" />
                  </div>
                </div>
              </div>
            ) : (
              /* Material 3 preview */
              <div
                className="overflow-hidden rounded-2xl border bg-[#fef7ff] text-[#1d1b20] shadow-[0_14px_34px_-14px_rgba(0,0,0,0.16)]"
                style={{ borderColor: 'rgba(0,0,0,0.12)' }}
              >
                <div className="flex items-center justify-between bg-[#f3edf7] px-4 py-3">
                  <div>
                    <div className="text-[14px] font-semibold text-[#1d1b20]">Atlas Studio</div>
                    <div className="font-mono text-[10.5px] text-[#6750a4]">waft-material — Material 3</div>
                  </div>
                  <span className="rounded-full bg-[#eaddff] px-2.5 py-0.5 text-[11px] font-semibold text-[#21005d]">
                    48 kHz
                  </span>
                </div>
                <div className="space-y-2.5 p-4">
                  <div className="flex items-center justify-between rounded-xl bg-[#f3edf7] px-3.5 py-2.5">
                    <div>
                      <div className="text-[12.5px] font-medium">Low-Latency Audio Pipeline</div>
                      <div className="text-[10.5px] text-zinc-600">Direct GPU & audio callbacks</div>
                    </div>
                    <button
                      type="button"
                      onClick={() => setLowLatency(!lowLatency)}
                      className="relative h-5 w-9 rounded-full transition"
                      style={{ background: lowLatency ? '#6750a4' : '#cac4d0' }}
                    >
                      <span
                        className="absolute top-0.5 size-4 rounded-full bg-white shadow transition-all"
                        style={{ left: lowLatency ? '18px' : '2px' }}
                      />
                    </button>
                  </div>
                  <div className="flex items-center justify-between rounded-xl bg-[#f3edf7] px-3.5 py-2.5">
                    <div>
                      <div className="text-[12.5px] font-medium">Spatial Binaural Compute</div>
                      <div className="text-[10.5px] text-zinc-600">Pure Rust valo + wgpu</div>
                    </div>
                    <button
                      type="button"
                      onClick={() => setSpatialDsp(!spatialDsp)}
                      className="relative h-5 w-9 rounded-full transition"
                      style={{ background: spatialDsp ? '#6750a4' : '#cac4d0' }}
                    >
                      <span
                        className="absolute top-0.5 size-4 rounded-full bg-white shadow transition-all"
                        style={{ left: spatialDsp ? '18px' : '2px' }}
                      />
                    </button>
                  </div>
                  <div className="flex justify-end pt-1">
                    <button
                      type="button"
                      onClick={() => setMidiSync(!midiSync)}
                      className="rounded-full bg-[#6750a4] px-4 py-1.5 text-[12px] font-medium text-white shadow-xs"
                    >
                      Export Build
                    </button>
                  </div>
                </div>
              </div>
            )}

            {/* Caption underneath interactive preview */}
            <div className="mt-4 flex items-center justify-between text-[11.5px]" style={{ color: 'var(--site-muted)' }}>
              <span className="inline-flex items-center gap-1.5 font-medium">
                <Sparkles className="size-3.5" style={{ color: 'var(--site-accent)' }} />
                Click any toggle or tab above — one widget model across kits
              </span>
              <span className="shrink-0 font-mono text-[11px] whitespace-nowrap" style={{ color: 'var(--site-subtle)' }}>
                0 unsafe blocks
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
