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
      'A direct, faithful port of Flutter’s widget, element, and render-object pipeline into Rust. Generational arena handles (Ref<T> and Handle<T>) replace garbage collection without borrow-checker gymnastics.',
    docsHref: '/docs/architecture',
    docsLabel: 'Architecture guide',
    crates: [
      { name: 'waft-widgets', summary: 'StatelessWidget, StatefulWidget, Navigator, Focus, Shortcuts', docsHref: '/docs/layout' },
      { name: 'waft-rendering', summary: 'RenderBox, RenderSliver, constraints, hit-testing, semantics', docsHref: '/docs/layout' },
      { name: 'waft-gestures', summary: 'Multi-touch GestureArena, tap, drag, scale, velocity trackers', docsHref: '/docs/architecture' },
      { name: 'waft-foundation', summary: 'Generational arena handles (Ref<T>, Handle<T>), async tasks', docsHref: '/docs/handles' },
    ],
    code: `pub trait StatelessWidget: Debug + 'static {
    fn build(&self, app: &mut App, cx: BuildContext) -> WidgetRef;
}

pub trait RenderBox: RenderObject {
    fn perform_layout(&self, app: &mut App, constraints: BoxConstraints) -> Size;
    fn paint(&self, app: &App, cx: &mut PaintingContext, offset: Offset);
}`,
  },
  {
    id: 'embedder',
    title: 'The Embedder Trait Boundary',
    tagline: 'dart:ui expressed as clean Rust traits — portable to any host',
    description:
      'Instead of C++ embedder ABI headers, Waft defines a single crate (waft-embedder) containing the Platform, View, and EmbedderClient traits. Swap hosts freely or embed Waft inside your own engine, game, or WASI sandbox.',
    docsHref: '/docs/embedder',
    docsLabel: 'Embedder guide',
    crates: [
      { name: 'waft-embedder', summary: 'Platform, View, EmbedderClient, Scene, Clipboard, Menus, HTTP', docsHref: '/docs/embedder' },
      { name: 'waft-embedder-apple', summary: 'macOS AppKit / Metal host & iOS UIKit / Metal host', docsHref: '/docs/embedder' },
      { name: 'waft-embedder-winit', summary: 'Windows, Linux (Wayland/X11) & cross-desktop host', docsHref: '/docs/embedder' },
      { name: 'waft-embedder-web', summary: 'Browser WebGPU + Wasm host & Android NDK / Vulkan host', docsHref: '/docs/embedder' },
    ],
    code: `/// The complete host contract in waft-embedder:
pub trait Platform {
    fn views(&self) -> Vec<ViewRef>;
    fn request_view(&self, config: WindowConfig) -> RequestId;
    fn clipboard(&self) -> Option<&dyn Clipboard>;
    fn menu_bar(&self) -> Option<&dyn MenuBar>;
}

pub trait EmbedderClient {
    fn begin_frame(&mut self, view: ViewId, frame: Frame) -> Scene;
    fn pointer_data(&mut self, view: ViewId, reports: &[PointerReport]);
}`,
  },
  {
    id: 'gpu',
    title: 'Pure Rust GPU Compositor (valo + wgpu)',
    tagline: 'Direct Metal, Vulkan, DX12, and WebGPU rendering with zero C++ dependencies',
    description:
      'Frames compile into retained Scene and Picture display lists rendered by valo on top of wgpu. Crisp shaped text, vector paths, rounded-rect shadows, backdrop blurs, and 120 Hz animations run straight on the GPU.',
    docsHref: '/docs/architecture',
    docsLabel: 'Rendering pipeline',
    crates: [
      { name: 'valo', summary: 'Pure-Rust 2D GPU renderer, text shaping, paths, and layer compositor', docsHref: '/docs/architecture' },
      { name: 'waft-painting', summary: 'Canvas, Paint, TextPainter, BoxDecoration, ImageCache', docsHref: '/docs/architecture' },
      { name: 'waft-animation', summary: 'AnimationController, Tween, CurvedAnimation, SpringSimulation', docsHref: '/docs/architecture' },
      { name: 'waft-physics', summary: 'iOS BouncingScrollSimulation, ClampingScrollSimulation, springs', docsHref: '/docs/architecture' },
    ],
    code: `let mut builder = SceneBuilder::new();
builder.push_offset(0.0, scroll_y, |layer| {
    layer.push_clip_rrect(card_rrect, Clip::AntiAlias, |card| {
        card.add_picture(Offset::ZERO, &recorded_picture);
    });
});
view.present(frame, builder.build());`,
  },
];

export function ArchitectureExplorer() {
  const [selectedId, setSelectedId] = useState<LayerId>('kits');
  const layer = LAYERS.find((l) => l.id === selectedId) ?? LAYERS[0];

  return (
    <section className="px-4 py-16 sm:px-6 sm:py-20">
      <div className="mx-auto max-w-[1160px]">
        {/* Section Heading */}
        <h2
          className="text-[32px] leading-[1.08] font-semibold tracking-[-0.03em] sm:text-[42px]"
          style={{ color: 'var(--site-ink)' }}
        >
          Ordinary Cargo crates.
          <br />
          Clean trait boundary at the bottom.
        </h2>

        {/* Interactive Stack Diagram (Left 5 cols) + Layer Inspector (Right 7 cols) */}
        <div className="mt-10 grid grid-cols-1 gap-6 lg:grid-cols-12 lg:items-stretch">
          {/* Left: 4 Stack Layers */}
          <div className="flex flex-col justify-between gap-3 lg:col-span-5">
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
                  className="group relative flex flex-1 flex-col justify-center rounded-2xl border p-5 text-left transition-all"
                  style={{
                    background: active ? 'var(--site-card)' : 'var(--site-surface)',
                    borderColor: active ? 'var(--site-ink)' : 'var(--site-hairline-strong)',
                    boxShadow: active ? '0 14px 34px -18px rgba(20, 20, 19, 0.14)' : 'none',
                  }}
                >
                  <div className="flex items-center justify-between gap-2">
                    <div
                      className="text-[16.5px] font-semibold tracking-[-0.015em]"
                      style={{ color: 'var(--site-ink)' }}
                    >
                      {item.title}
                    </div>
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

          {/* Right: Active Layer Deep Dive */}
          <div
            className="flex flex-col justify-between rounded-2xl border p-6 sm:p-8 lg:col-span-7"
            style={{
              background: 'var(--site-card)',
              borderColor: 'var(--site-hairline-strong)',
            }}
          >
            <div>
              <div className="flex flex-wrap items-center justify-between gap-2">
                <h3
                  className="text-[20px] font-semibold tracking-[-0.02em]"
                  style={{ color: 'var(--site-ink)' }}
                >
                  {layer.title}
                </h3>
                <Link
                  href={layer.docsHref}
                  className="inline-flex items-center gap-1 text-[13px] font-medium transition hover:opacity-75"
                  style={{ color: 'var(--site-ink)' }}
                >
                  <span>{layer.docsLabel}</span>
                  <ArrowUpRight className="size-3.5" />
                </Link>
              </div>

              <p
                className="mt-3 text-[15px] leading-[1.6]"
                style={{ color: 'var(--site-muted)' }}
              >
                {layer.description}
              </p>

              {/* Crates Grid in this Layer */}
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
