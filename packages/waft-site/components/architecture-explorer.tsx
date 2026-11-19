'use client';

import { useState } from 'react';
import Link from 'next/link';
import { ArrowUpRight, Box, Cpu, Layers, Sparkles } from 'lucide-react';
import { RustCodeBlock } from '@/components/code-highlight';

type LayerId = 'kits' | 'framework' | 'embedder' | 'gpu';

interface CrateSpec {
  name: string;
  summary: string;
  docsHref: string;
}

interface LayerSpec {
  id: LayerId;
  title: string;
  tagline: string;
  description: string;
  docsHref: string;
  docsLabel: string;
  crates: CrateSpec[];
  code: string;
}

const LAYERS: LayerSpec[] = [
  {
    id: 'kits',
    title: 'Design System Kits',
    tagline: 'Desktop WinUI, mobile Cupertino, Material 3, and Markdown',
    description:
      'Ordinary Cargo crates built entirely on Waft widgets. Use a full kit out of the box, mix controls across kits, or fork a single crate to build your own design system.',
    docsHref: '/docs/kits',
    docsLabel: 'UI kits guide',
    crates: [
      { name: 'waft-winui', summary: 'Fluent / WinUI 3 controls, NavigationView, dialogs, menus', docsHref: '/docs/kits' },
      { name: 'waft-cupertino', summary: 'iOS/macOS controls, sliver nav bars, sheets, physics scroll', docsHref: '/docs/kits' },
      { name: 'waft-material', summary: 'Material 3 AppBar, Scaffold, NavigationBar, cards, themes', docsHref: '/docs/kits' },
      { name: 'waft-markdown', summary: 'Selectable, syntax-highlighted Markdown & code blocks', docsHref: '/docs/kits' },
    ],
    code: `use waft_winui::{FluentApp, NavigationView, ToggleSwitch};
use waft_markdown::Markdown;

// Every control is a regular Rust struct implementing Widget
NavigationView::new()
    .pane_title("Documentation")
    .item("Overview", Markdown::new(include_str!("../README.md")))
    .item("Settings", ToggleSwitch::new(true, |app, on| { /* ... */ }))`,
  },
  {
    id: 'framework',
    title: 'Flutter-Model Widget & Layout Engine',
    tagline: 'Widgets, flex/sliver layout, gestures, IME text input, and arena handles',
    description:
      'A direct, faithful port of Flutter’s widget, element, and render-object pipeline into safe Rust. Generational arena handles (Ref<T> and Handle<T>) replace garbage collection without borrow-checker gymnastics.',
    docsHref: '/docs/architecture',
    docsLabel: 'Architecture guide',
    crates: [
      { name: 'waft-widgets', summary: 'StatelessWidget, StatefulWidget, Navigator, Focus, Shortcuts', docsHref: '/docs/layout' },
      { name: 'waft-rendering', summary: 'Flex, Stack, Sliver, CustomPainter, hit-testing & layout', docsHref: '/docs/layout' },
      { name: 'waft-gestures', summary: 'Multi-touch gesture arena, pan/pinch/tap disambiguation, fling', docsHref: '/docs/architecture' },
      { name: 'waft-services', summary: 'Full CJK IME composition, hardware keys, clipboard, menus', docsHref: '/docs/text-input' },
    ],
    code: `impl StatelessWidget for ToolbarRow {
    fn build(&self, _app: &mut App, _cx: BuildContext) -> WidgetRef {
        Row::new()
            .spacing(12.0)
            .child(Expanded::new(TextBox::new(self.query.clone())))
            .child(Button::accent("Run", Listener::new(|app| { /* ... */ })))
            .into_widget()
    }
}`,
  },
  {
    id: 'embedder',
    title: 'The Embedder Trait Boundary',
    tagline: 'dart:ui expressed as clean Rust traits — portable to any host',
    description:
      'Everything the framework needs from the operating system sits behind waft-embedder. Use waft-embedder-default for macOS, Windows, Linux, iOS, Android, and Web, or implement the traits for your own engine or WASI host.',
    docsHref: '/docs/embedder',
    docsLabel: 'Custom embedder guide',
    crates: [
      { name: 'waft-embedder', summary: 'Platform, Window, View, PictureRecorder & Event traits', docsHref: '/docs/embedder' },
      { name: 'waft-embedder-default', summary: 'Production host for macOS, Windows, Linux, iOS, Android, Web', docsHref: '/docs/embedder' },
      { name: 'waft-shell', summary: 'Frame scheduling, pointer/keyboard dispatch, view lifecycle', docsHref: '/docs/embedder' },
      { name: 'waft-test', summary: 'Headless embedder for millisecond cargo test execution', docsHref: '/docs/testing' },
    ],
    code: `pub trait Platform {
    fn create_window(&mut self, config: WindowConfig) -> Result<ViewId>;
    fn request_frame(&mut self, view: ViewId);
    fn set_text_input_client(&mut self, view: ViewId, client: Option<ImeConfig>);
    fn clipboard(&mut self) -> &mut dyn ClipboardProvider;
}`,
  },
  {
    id: 'gpu',
    title: 'Pure Rust GPU Compositor (valo + wgpu)',
    tagline: 'Direct Metal, Vulkan, DX12, and WebGPU rendering with zero C++ dependencies',
    description:
      'Waft records vector display lists and composites layers directly through valo and wgpu. No Skia C++ build, no Chromium runtime, and minimal memory and CPU overhead.',
    docsHref: '/docs/architecture',
    docsLabel: 'Rendering pipeline',
    crates: [
      { name: 'valo', summary: 'Pure Rust 2D vector, text shaping, and layer compositor on wgpu', docsHref: '/docs/architecture' },
      { name: 'wgpu', summary: 'Native Metal, Vulkan, DirectX 12, and browser WebGPU backends', docsHref: '/docs/architecture' },
      { name: 'waft-painting', summary: 'Canvas, Paint, Path, RRect, TextPainter, Image shaders', docsHref: '/docs/architecture' },
      { name: 'waft-animation', summary: 'Vsync ticker, spring simulations, cubic curves & transitions', docsHref: '/docs/architecture' },
    ],
    code: `impl CustomPainter for WaveformPainter {
    fn paint(&self, canvas: &mut Canvas, size: Size) {
        let paint = Paint::stroke(Color::from_rgb(0xC8, 0x4B, 0x11), 2.0);
        canvas.draw_path(&self.build_spline(size), &paint);
    }
}`,
  },
];

export function ArchitectureExplorer() {
  const [selectedId, setSelectedId] = useState<LayerId>('kits');
  const layer = LAYERS.find((l) => l.id === selectedId) ?? LAYERS[0];

  return (
    <section className="px-4 py-16 sm:px-6 sm:py-20">
      <div className="mx-auto max-w-[1160px]">
        <div className="grid grid-cols-1 gap-6 lg:grid-cols-12 lg:items-end">
          <div className="lg:col-span-7">
            <h2
              className="text-[32px] leading-[1.08] font-semibold tracking-[-0.03em] sm:text-[42px]"
              style={{ color: 'var(--site-ink)' }}
            >
              Ordinary Cargo crates.
              <br />
              Clean trait boundary at the bottom.
            </h2>
          </div>
          <p
            className="text-[15.5px] leading-[1.6] lg:col-span-5"
            style={{ color: 'var(--site-muted)' }}
          >
            No monolithic black box and no C++ build scripts. Every layer is a normal Rust package with{' '}
            <code className="rounded bg-black/[0.05] px-1.5 py-0.5 font-mono text-[13px] text-[#141413]">
              #![forbid(unsafe_code)]
            </code>{' '}
            that you can inspect, replace, or port to a new host.
          </p>
        </div>

        {/* Interactive 12-col Blueprint Explorer */}
        <div className="mt-10 grid grid-cols-1 gap-6 lg:grid-cols-12">
          {/* Left 5 cols: Stack of 4 clickable architectural layers */}
          <div className="flex flex-col gap-3 lg:col-span-5">
            {LAYERS.map((item) => {
              const active = item.id === selectedId;
              const Icon =
                item.id === 'kits'
                  ? Sparkles
                  : item.id === 'framework'
                    ? Layers
                    : item.id === 'embedder'
                      ? Box
                      : Cpu;
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => setSelectedId(item.id)}
                  className="group relative flex flex-col rounded-2xl border p-4 text-left transition-all sm:p-5"
                  style={{
                    background: active ? 'var(--site-card)' : 'var(--site-surface)',
                    borderColor: active ? 'var(--site-ink)' : 'var(--site-hairline-strong)',
                    boxShadow: active ? '0 14px 32px -18px rgba(20,20,19,0.16)' : 'none',
                  }}
                >
                  <div className="flex w-full items-center justify-between gap-3">
                    <span
                      className="text-[16px] font-semibold tracking-[-0.015em]"
                      style={{ color: 'var(--site-ink)' }}
                    >
                      {item.title}
                    </span>
                    <Icon
                      className="size-4 shrink-0"
                      style={{ color: active ? 'var(--site-accent)' : 'var(--site-subtle)' }}
                    />
                  </div>
                  <div
                    className="mt-1 text-[13px] leading-[1.45]"
                    style={{ color: 'var(--site-muted)' }}
                  >
                    {item.tagline}
                  </div>
                </button>
              );
            })}
          </div>

          {/* Right 7 cols: Deep dive on the selected layer */}
          <div
            className="flex flex-col justify-between rounded-2xl border p-6 sm:p-7 lg:col-span-7"
            style={{
              background: 'var(--site-card)',
              borderColor: 'var(--site-hairline-strong)',
              boxShadow: '0 20px 50px -28px rgba(20,20,19,0.12)',
            }}
          >
            <div>
              <div className="flex flex-wrap items-center justify-between gap-2">
                <h3
                  className="text-[18px] font-semibold tracking-[-0.015em]"
                  style={{ color: 'var(--site-ink)' }}
                >
                  {layer.title}
                </h3>
                <Link
                  href={layer.docsHref}
                  className="inline-flex items-center gap-1 text-[13px] font-medium transition hover:opacity-75"
                  style={{ color: 'var(--site-ink)' }}
                >
                  {layer.docsLabel}
                  <ArrowUpRight className="size-3.5" />
                </Link>
              </div>

              <p
                className="mt-2.5 text-[14.5px] leading-[1.6]"
                style={{ color: 'var(--site-muted)' }}
              >
                {layer.description}
              </p>

              {/* 2x2 grid of Cargo crates in this layer */}
              <div className="mt-5 grid grid-cols-1 gap-2.5 sm:grid-cols-2">
                {layer.crates.map((c) => (
                  <Link
                    key={c.name}
                    href={c.docsHref}
                    className="group rounded-xl border p-3.5 transition hover:border-black/30"
                    style={{
                      background: 'var(--site-surface)',
                      borderColor: 'var(--site-hairline)',
                    }}
                  >
                    <div className="flex items-center justify-between">
                      <span
                        className="font-mono text-[13px] font-semibold"
                        style={{ color: 'var(--site-ink)' }}
                      >
                        {c.name}
                      </span>
                      <ArrowUpRight
                        className="size-3.5 transition group-hover:translate-x-0.5 group-hover:-translate-y-0.5"
                        style={{ color: 'var(--site-subtle)' }}
                      />
                    </div>
                    <p
                      className="mt-1 text-[12px] leading-[1.45]"
                      style={{ color: 'var(--site-muted)' }}
                    >
                      {c.summary}
                    </p>
                  </Link>
                ))}
              </div>
            </div>

            {/* Highlighted code snippet for this layer */}
            <div className="mt-5">
              <RustCodeBlock code={layer.code} />
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
