'use client';

import { useState } from 'react';
import Link from 'next/link';
import { ArrowUpRight, Layers, Box, Cpu, Sparkles } from 'lucide-react';

interface LayerSpec {
  id: string;
  step: string;
  title: string;
  subtitle: string;
  description: string;
  crates: readonly { name: string; href: string; role: string }[];
  codeTitle: string;
  code: string;
  docsHref: string;
  docsLabel: string;
}

const LAYERS: readonly LayerSpec[] = [
  {
    id: 'kits',
    step: 'LAYER 01',
    title: 'Design System Kits',
    subtitle: 'Desktop WinUI, mobile Cupertino, Material 3, and Markdown',
    description:
      'Ordinary Cargo crates built entirely on Waft widgets. Use a full kit out of the box, mix controls across kits, or fork a single crate to build your own company design system.',
    crates: [
      { name: 'waft-winui', href: '/docs/kits', role: 'Fluent / WinUI 3 controls, NavigationView, dialogs, menus' },
      { name: 'waft-cupertino', href: '/docs/kits', role: 'iOS/macOS controls, sliver nav bars, sheets, physics scroll' },
      { name: 'waft-material', href: '/docs/kits', role: 'Material 3 AppBar, Scaffold, NavigationBar, cards, themes' },
      { name: 'waft-markdown', href: '/docs/kits', role: 'Selectable, syntax-highlighted Markdown & code blocks' },
    ],
    codeTitle: 'Mix or customize any kit crate',
    code: `use waft_winui::{FluentApp, NavigationView, ToggleSwitch};
use waft_markdown::Markdown;

// Every control is a regular Rust struct implementing Widget
NavigationView::new()
    .pane_title("Documentation")
    .item("Overview", Markdown::new(include_str!("../README.md")))
    .item("Settings", ToggleSwitch::new(true, |app, on| { /* ... */ }))`,
    docsHref: '/docs/kits',
    docsLabel: 'Explore the UI kits guide',
  },
  {
    id: 'framework',
    step: 'LAYER 02',
    title: 'Flutter-Model Widget & Layout Engine',
    subtitle: 'Widgets, flex/sliver layout, gestures, IME text input, and arena handles',
    description:
      'Proven Flutter architecture ported idiomatic to Rust with #![forbid(unsafe_code)]. Single-pass constraint layout, multi-touch gesture arenas, full IME composition, and focus traversal.',
    crates: [
      { name: 'waft-widgets', href: '/docs/layout', role: 'StatelessWidget, StatefulWidget,Navigator, Focus, Shortcuts' },
      { name: 'waft-rendering', href: '/docs/layout', role: 'RenderBox & RenderSliver constraint layout & hit testing' },
      { name: 'waft-gestures', href: '/docs/text-input', role: 'Tap, drag, pinch, fling physics, and pointer arena disambiguation' },
      { name: 'waft-foundation', href: '/docs/handles', role: 'App arena, Ref<T> & Handle<T>, async tasks, headless AppCell' },
    ],
    codeTitle: 'Single-pass layout & multi-touch gestures',
    code: `use waft::{Column, Container, CrossAxisAlignment, GestureDetector, Padding, Text};

GestureDetector::new()
    .on_tap(|app| { /* pointer & touch arena winner */ })
    .child(
        Container::new()
            .padding(Padding::symmetric(16.0, 12.0))
            .child(Column::new().cross_axis_alignment(CrossAxisAlignment::Start)),
    )`,
    docsHref: '/docs/architecture',
    docsLabel: 'Read the framework architecture',
  },
  {
    id: 'embedder',
    step: 'LAYER 03',
    title: 'The Embedder Trait Boundary',
    subtitle: 'dart:ui expressed as clean Rust traits — portable to any host in a weekend',
    description:
      'The entire framework talks to the host OS through waft-embedder traits (PlatformDispatcher, View, SceneBuilder, TextInputClient). Porting Waft to a new OS, game engine, or sandbox never touches widget code.',
    crates: [
      { name: 'waft-embedder', href: '/docs/embedder', role: 'Host-agnostic window, input, IME, clipboard, and scene traits' },
      { name: 'waft-embedder-winit', href: '/docs/embedder', role: 'Desktop windowing for macOS, Windows, and Linux (Wayland/X11)' },
      { name: 'waft-embedder-apple', href: '/docs/embedder', role: 'UIKit & AppKit runner with CADisplayLink & UITextInput IME' },
      { name: 'waft-embedder-android', href: '/docs/embedder', role: 'NativeActivity / GameActivity runner with Choreographer & JNI IME' },
    ],
    codeTitle: 'Clean host contract in waft-embedder',
    code: `pub trait EmbedderHost {
    fn schedule_frame(&mut self);
    fn render_scene(&mut self, scene: &Scene, viewport: Size);
    fn set_text_input_state(&mut self, state: TextInputState);
    fn set_cursor(&mut self, cursor: CursorIcon);
}
// Implement this trait boundary to embed Waft inside any host window or runtime.`,
    docsHref: '/docs/embedder',
    docsLabel: 'Read the embedder & porting guide',
  },
  {
    id: 'gpu',
    step: 'LAYER 04',
    title: 'Pure Rust GPU Compositor (valo + wgpu)',
    subtitle: 'Direct Metal, Vulkan, DX12, and WebGPU rendering with zero C++ dependencies',
    description:
      'valo rasterizes display lists, rounded clip layers, shadows, and shaped glyphs directly onto wgpu command buffers. No Skia C++ build chain, no browser engine, and minimal memory footprint.',
    crates: [
      { name: 'valo', href: '/docs/reference', role: '2D vector & layer compositor built directly on wgpu' },
      { name: 'waft-painting', href: '/docs/reference', role: 'Canvas, RRect, BoxShadow, gradients, and cosmic-text shaping' },
      { name: 'waft-embedder-web', href: '/docs/embedder', role: 'WebGPU / WebGL2 browser canvas runner' },
      { name: 'waft-test', href: '/docs/testing', role: 'Headless software & offscreen harness for CI without a GPU' },
    ],
    codeTitle: 'Direct GPU canvas drawing when you need custom visuals',
    code: `fn paint(&self, ctx: &mut PaintingContext, offset: Offset) {
    let rect = Rect::from_origin_size(offset, self.size());
    ctx.canvas.draw_rrect(
        RRect::from_rect_radius(rect, 10.0),
        &Paint::fill(Color::from_rgb8(0, 95, 184)),
    );
}`,
    docsHref: '/docs/reference',
    docsLabel: 'Browse all crate references',
  },
];

export function ArchitectureExplorer() {
  const [activeId, setActiveId] = useState<string>('kits');
  const current = LAYERS.find((l) => l.id === activeId) ?? LAYERS[0];

  return (
    <section className="px-6 py-20 sm:py-24">
      <div className="mx-auto max-w-[1160px]">
        {/* Section header */}
        <div className="flex flex-col justify-between gap-4 md:flex-row md:items-end">
          <div>
            <p
              className="font-mono text-[12px] font-semibold uppercase tracking-[0.12em]"
              style={{ color: 'var(--site-accent)' }}
            >
              01 — Modular Rust Architecture
            </p>
            <h2
              className="mt-2 text-[clamp(1.85rem,3.6vw,2.75rem)] leading-[1.08] font-semibold tracking-[-0.03em]"
              style={{ color: 'var(--site-ink)' }}
            >
              Ordinary Cargo crates.
              <br className="hidden sm:inline" /> Clean trait boundary at the bottom.
            </h2>
          </div>
          <p className="max-w-[440px] text-[15px] leading-[1.6]" style={{ color: 'var(--site-muted)' }}>
            No monolithic black box and no C++ build scripts. Every layer is a normal Rust package with{' '}
            <code className="rounded bg-black/[0.05] px-1.5 py-0.5 font-mono text-[13px] dark:bg-white/[0.08]">
              #![forbid(unsafe_code)]
            </code>{' '}
            that you can inspect, replace, or port to a new host.
          </p>
        </div>

        {/* Interactive 12-col blueprint grid */}
        <div className="mt-10 grid grid-cols-1 gap-5 lg:grid-cols-12">
          {/* Left 5 cols: Stack selector */}
          <div className="flex flex-col gap-2.5 lg:col-span-5">
            {LAYERS.map((layer, idx) => {
              const selected = layer.id === current.id;
              const Icon = idx === 0 ? Sparkles : idx === 1 ? Layers : idx === 2 ? Box : Cpu;
              return (
                <button
                  key={layer.id}
                  type="button"
                  onClick={() => setActiveId(layer.id)}
                  className="group relative flex flex-col rounded-2xl border p-4 text-left transition"
                  style={{
                    background: selected ? 'var(--site-card)' : 'var(--site-surface)',
                    borderColor: selected ? 'var(--site-ink)' : 'var(--site-hairline)',
                    boxShadow: selected ? '0 14px 34px -18px rgba(20,20,19,0.16)' : 'none',
                  }}
                >
                  <div className="flex items-center justify-between">
                    <span
                      className="font-mono text-[11px] font-semibold tracking-[0.08em]"
                      style={{ color: selected ? 'var(--site-accent)' : 'var(--site-subtle)' }}
                    >
                      {layer.step}
                    </span>
                    <Icon
                      className="size-4 transition"
                      style={{ color: selected ? 'var(--site-ink)' : 'var(--site-subtle)' }}
                    />
                  </div>
                  <div
                    className="mt-1 text-[16px] font-semibold tracking-[-0.015em]"
                    style={{ color: 'var(--site-ink)' }}
                  >
                    {layer.title}
                  </div>
                  <div className="mt-0.5 text-[13px] leading-[1.45]" style={{ color: 'var(--site-muted)' }}>
                    {layer.subtitle}
                  </div>
                </button>
              );
            })}
          </div>

          {/* Right 7 cols: Selected Layer Detail Panel */}
          <div
            className="flex flex-col justify-between rounded-2xl border p-6 sm:p-7 lg:col-span-7"
            style={{
              background: 'var(--site-card)',
              borderColor: 'var(--site-hairline-strong)',
            }}
          >
            <div>
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span
                  className="rounded-md px-2.5 py-1 font-mono text-[11px] font-semibold"
                  style={{
                    background: 'var(--site-surface)',
                    color: 'var(--site-accent)',
                  }}
                >
                  {current.step} — {current.title}
                </span>
                <Link
                  href={current.docsHref}
                  className="inline-flex items-center gap-1 text-[13px] font-medium transition hover:opacity-75"
                  style={{ color: 'var(--site-ink)' }}
                >
                  {current.docsLabel}
                  <ArrowUpRight className="size-3.5" />
                </Link>
              </div>

              <p className="mt-3.5 text-[15px] leading-[1.6]" style={{ color: 'var(--site-muted)' }}>
                {current.description}
              </p>

              {/* Crate table */}
              <div className="mt-5 grid grid-cols-1 gap-2.5 sm:grid-cols-2">
                {current.crates.map((crate) => (
                  <Link
                    key={crate.name}
                    href={crate.href}
                    className="group rounded-xl border p-3 transition hover:border-black/30 dark:hover:border-white/30"
                    style={{
                      background: 'var(--site-surface)',
                      borderColor: 'var(--site-hairline)',
                    }}
                  >
                    <div className="flex items-center justify-between">
                      <code className="font-mono text-[12.5px] font-semibold" style={{ color: 'var(--site-ink)' }}>
                        {crate.name}
                      </code>
                      <ArrowUpRight
                        className="size-3.5 opacity-40 transition group-hover:opacity-100"
                        style={{ color: 'var(--site-ink)' }}
                      />
                    </div>
                    <p className="mt-1 text-[12px] leading-[1.45]" style={{ color: 'var(--site-muted)' }}>
                      {crate.role}
                    </p>
                  </Link>
                ))}
              </div>
            </div>

            {/* Code snippet for this layer */}
            <div className="mt-6">
              <div className="mb-2 flex items-center justify-between font-mono text-[11px]" style={{ color: 'var(--site-subtle)' }}>
                <span>{current.codeTitle}</span>
                <span>Rust 2024</span>
              </div>
              <pre
                className="overflow-x-auto rounded-xl border p-4 font-mono text-[12px] leading-[19px]"
                style={{
                  background: '#fcfbf9',
                  borderColor: 'var(--site-hairline)',
                  color: '#1c1b18',
                }}
              >
                <code>{current.code}</code>
              </pre>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
