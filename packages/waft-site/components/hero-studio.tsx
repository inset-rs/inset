'use client';

import { useState } from 'react';
import { Check, Copy, ExternalLink, Layers, Monitor, Smartphone } from 'lucide-react';
import { galleryUrls } from '@/lib/shared';
import { RustCodeBlock } from '@/components/code-highlight';

type KitId = 'winui' | 'cupertino' | 'material';

interface KitSpec {
  id: KitId;
  label: string;
  crateName: string;
  galleryUrl: string;
  galleryLabel: string;
  code: string;
}

const KITS: KitSpec[] = [
  {
    id: 'winui',
    label: 'Fluent / WinUI',
    crateName: 'waft-winui',
    galleryUrl: galleryUrls.winui,
    galleryLabel: 'WinUI Gallery',
    code: `use waft::{App, BuildContext, IntoWidget, StatelessWidget, WidgetRef, run_app};
use waft_winui::{Button, FluentApp, NavigationView, ToggleSwitch};

#[waft::main]
fn main(app: &mut App) {
    run_app(app, FluentApp::new().home(Studio).into_widget());
}

impl StatelessWidget for Studio {
    fn build(&self, _app: &mut App, _cx: BuildContext) -> WidgetRef {
        NavigationView::new()
            .pane_title("Atlas Studio")
            .content(ToggleSwitch::new(true, |app, on| { /* ... */ }))
            .footer(Button::accent("Export Build", Listener::new(|app| {})))
            .into_widget()
    }
}`,
  },
  {
    id: 'cupertino',
    label: 'Cupertino',
    crateName: 'waft-cupertino',
    galleryUrl: galleryUrls.cupertino,
    galleryLabel: 'Cupertino Gallery',
    code: `use waft::{App, BuildContext, CustomScrollView, IntoWidget, StatelessWidget, WidgetRef, run_app};
use waft_cupertino::{CupertinoApp, CupertinoSliverNavigationBar, CupertinoSwitch};

#[waft::main]
fn main(app: &mut App) {
    run_app(app, CupertinoApp::new().home(Studio).into_widget());
}

impl StatelessWidget for Studio {
    fn build(&self, _app: &mut App, _cx: BuildContext) -> WidgetRef {
        CustomScrollView::new()
            .sliver(CupertinoSliverNavigationBar::new("Atlas Studio"))
            .sliver(CupertinoSwitch::new(true, |app, on| { /* ... */ }))
            .into_widget()
    }
}`,
  },
  {
    id: 'material',
    label: 'Material 3',
    crateName: 'waft-material',
    galleryUrl: galleryUrls.material,
    galleryLabel: 'Material Gallery',
    code: `use waft::{App, BuildContext, IntoWidget, StatelessWidget, WidgetRef, run_app};
use waft_material::{AppBar, Card, FilledButton, MaterialApp, Scaffold, Switch};

#[waft::main]
fn main(app: &mut App) {
    run_app(app, MaterialApp::new().home(Studio).into_widget());
}

impl StatelessWidget for Studio {
    fn build(&self, _app: &mut App, _cx: BuildContext) -> WidgetRef {
        Scaffold::new()
            .app_bar(AppBar::new("Atlas Studio"))
            .body(Card::elevated().child(Switch::new(true)))
            .floating_action(FilledButton::new("Export Build"))
            .into_widget()
    }
}`,
  },
];

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
                    background: selected ? 'var(--site-card)' : 'transparent',
                    color: selected ? 'var(--site-ink)' : 'var(--site-muted)',
                    boxShadow: selected ? '0 1px 3px rgba(0,0,0,0.08), inset 0 0 0 1px rgba(20,20,19,0.12)' : 'none',
                  }}
                >
                  <Icon className="size-3.5" style={{ color: selected ? 'var(--site-accent)' : 'currentColor' }} />
                  <span>{k.label}</span>
                  <span
                    className="rounded px-1.5 py-0.5 font-mono text-[10.5px]"
                    style={{
                      background: selected ? 'var(--site-accent-soft)' : 'rgba(20,20,19,0.05)',
                      color: selected ? 'var(--site-accent)' : 'var(--site-subtle)',
                    }}
                  >
                    {k.crateName}
                  </span>
                </button>
              );
            })}
          </div>

          {/* Right: Live WebGPU Gallery & CLI command copy */}
          <div className="flex flex-wrap items-center gap-2">
            <a
              href={spec.galleryUrl}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-1.5 rounded-lg border px-2.5 py-1 text-[12px] font-medium transition hover:opacity-80"
              style={{
                background: 'var(--site-card)',
                borderColor: 'var(--site-hairline-strong)',
                color: 'var(--site-ink)',
              }}
            >
              <span>{spec.galleryLabel}</span>
              <ExternalLink className="size-3" />
            </a>
            <button
              type="button"
              onClick={copyCli}
              className="inline-flex items-center gap-2 rounded-lg border px-2.5 py-1 font-mono text-[11.5px] transition hover:opacity-85"
              style={{
                background: 'var(--site-card)',
                borderColor: 'var(--site-hairline-strong)',
                color: 'var(--site-ink)',
              }}
              title="Copy quickstart command"
            >
              <span style={{ color: 'var(--site-accent)' }}>$</span>
              <span>cargo waft new notes</span>
              {copiedCmd ? <Check className="size-3.5 text-emerald-600" /> : <Copy className="size-3 opacity-60" />}
            </button>
          </div>
        </div>

        {/* Split body: Left 7 cols Rust Source, Right 5 cols Live Kit Preview */}
        <div className="grid grid-cols-1 lg:grid-cols-12">
          {/* Left: Rust Code Pane */}
          <div
            className="border-b p-4 sm:p-6 lg:col-span-7 lg:border-r lg:border-b-0"
            style={{
              background: '#fcfbf9',
              borderColor: 'var(--site-hairline)',
            }}
          >
            <RustCodeBlock
              code={spec.code}
              showLineNumbers
              className="!rounded-none !border-0 !bg-transparent !p-0"
            />
          </div>

          {/* Right: Interactive UI Kit Preview Surface */}
          <div
            className="site-dot-matrix flex flex-col justify-center p-5 sm:p-6 lg:col-span-5"
            style={{ background: 'var(--site-surface)' }}
          >
            {activeKit === 'winui' ? (
              <div
                className="overflow-hidden rounded-xl border bg-white text-[#18181b] shadow-[0_14px_34px_-14px_rgba(0,0,0,0.16)]"
                style={{ borderColor: 'rgba(20,20,19,0.14)' }}
              >
                <div className="flex items-center justify-between border-b border-black/[0.07] bg-[#f6f6f7] px-3.5 py-2">
                  <div className="flex items-center gap-1.5">
                    <span className="size-2.5 rounded-full bg-[#ff5f57]" />
                    <span className="size-2.5 rounded-full bg-[#febc2e]" />
                    <span className="size-2.5 rounded-full bg-[#28c840]" />
                    <span className="ml-2 text-[11.5px] font-semibold text-zinc-800">Atlas Studio</span>
                  </div>
                  <span className="rounded bg-[#005fb8]/10 px-1.5 py-0.5 font-mono text-[10px] font-medium text-[#005fb8]">
                    FluentApp
                  </span>
                </div>

                <div className="grid grid-cols-12">
                  <div className="col-span-4 border-r border-black/[0.06] bg-[#fafafa] p-2 text-[11.5px]">
                    {(
                      [
                        ['audio', 'Audio Engine'],
                        ['shortcuts', 'Shortcuts'],
                        ['targets', 'Build Targets'],
                      ] as const
                    ).map(([id, title]) => (
                      <button
                        key={id}
                        type="button"
                        onClick={() => setActiveNav(id)}
                        className="mb-1 flex w-full items-center gap-1.5 rounded-md px-2 py-1.5 text-left font-medium transition"
                        style={{
                          background: activeNav === id ? 'rgba(0, 95, 184, 0.1)' : 'transparent',
                          color: activeNav === id ? '#005fb8' : '#52525b',
                        }}
                      >
                        <span
                          className="h-3 w-0.5 rounded-full"
                          style={{ background: activeNav === id ? '#005fb8' : 'transparent' }}
                        />
                        {title}
                      </button>
                    ))}
                  </div>

                  <div className="col-span-8 space-y-2.5 p-3.5">
                    <div className="flex items-center justify-between rounded-lg border border-black/[0.07] bg-[#fcfcfd] px-3 py-2">
                      <div>
                        <div className="text-[12px] font-semibold">Low-Latency Audio</div>
                        <div className="text-[10.5px] text-zinc-500">Exclusive 48 kHz CoreAudio / WASAPI</div>
                      </div>
                      <button
                        type="button"
                        onClick={() => setLowLatency(!lowLatency)}
                        aria-label="Toggle Low-Latency Audio"
                        className="relative h-5 w-9 rounded-full transition"
                        style={{ background: lowLatency ? '#005fb8' : '#d4d4d8' }}
                      >
                        <span
                          className="absolute top-0.5 size-4 rounded-full bg-white shadow transition-all"
                          style={{ left: lowLatency ? '18px' : '2px' }}
                        />
                      </button>
                    </div>

                    <div className="flex items-center justify-between rounded-lg border border-black/[0.07] bg-[#fcfcfd] px-3 py-2">
                      <div>
                        <div className="text-[12px] font-semibold">Spatial Binaural DSP</div>
                        <div className="text-[10.5px] text-zinc-500">wgpu compute shader convolution</div>
                      </div>
                      <button
                        type="button"
                        onClick={() => setSpatialDsp(!spatialDsp)}
                        aria-label="Toggle Spatial Binaural DSP"
                        className="relative h-5 w-9 rounded-full transition"
                        style={{ background: spatialDsp ? '#005fb8' : '#d4d4d8' }}
                      >
                        <span
                          className="absolute top-0.5 size-4 rounded-full bg-white shadow transition-all"
                          style={{ left: spatialDsp ? '18px' : '2px' }}
                        />
                      </button>
                    </div>

                    <div className="flex items-center justify-between pt-1">
                      <span className="font-mono text-[10.5px] text-zinc-500">
                        {lowLatency && spatialDsp ? 'DSP buffer: 2.6 ms' : 'DSP buffer: 5.3 ms'}
                      </span>
                      <button
                        type="button"
                        onClick={() => setMidiSync(!midiSync)}
                        className="rounded-md bg-[#005fb8] px-3 py-1 text-[11.5px] font-medium text-white shadow-xs transition hover:bg-[#00529e]"
                      >
                        {midiSync ? 'Build Exported ✓' : 'Export Build'}
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            ) : activeKit === 'cupertino' ? (
              <div
                className="mx-auto w-full max-w-[300px] overflow-hidden rounded-[24px] border bg-[#f2f2f7] text-[#1c1c1e] shadow-[0_14px_34px_-14px_rgba(0,0,0,0.18)]"
                style={{ borderColor: 'rgba(20,20,19,0.16)' }}
              >
                <div className="border-b border-black/[0.07] bg-white/90 px-4 pt-2.5 pb-2 backdrop-blur">
                  <div className="flex items-center justify-between text-[10.5px] font-semibold text-zinc-500">
                    <span>9:41</span>
                    <span className="font-mono text-[10px] text-[#007aff]">CupertinoApp</span>
                    <span>100%</span>
                  </div>
                  <div className="mt-2 flex items-baseline justify-between">
                    <span className="text-[19px] font-bold tracking-tight text-black">Atlas Studio</span>
                    <span className="text-[12px] font-medium text-[#007aff]">Done</span>
                  </div>
                </div>

                <div className="p-3.5">
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

                  <div className="mt-3 flex justify-center">
                    <div className="h-1 w-24 rounded-full bg-black/25" />
                  </div>
                </div>
              </div>
            ) : (
              <div
                className="overflow-hidden rounded-2xl border bg-[#fef7ff] text-[#1d1b20] shadow-[0_14px_34px_-14px_rgba(0,0,0,0.16)]"
                style={{ borderColor: 'rgba(0,0,0,0.12)' }}
              >
                <div className="flex items-center justify-between bg-[#f3edf7] px-4 py-3">
                  <div>
                    <div className="text-[14px] font-semibold text-[#1d1b20]">Atlas Studio</div>
                    <div className="font-mono text-[10.5px] text-[#6750a4]">MaterialApp</div>
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
          </div>
        </div>
      </div>
    </div>
  );
}
