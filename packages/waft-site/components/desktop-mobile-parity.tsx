import Link from 'next/link';
import { ExternalLink, Monitor, Smartphone, Gauge, Cpu, ShieldCheck, ArrowUpRight } from 'lucide-react';
import { galleryUrls } from '@/lib/shared';

const DESKTOP_SYSTEMS = [
  {
    name: 'NavigationView, TabView & SplitView',
    detail: 'Collapsible panes, multi-document tab strips, and breadcrumb headers.',
  },
  {
    name: 'MenuBar, ContextMenu & CommandBar',
    detail: 'Nested accelerators, hardware keychord routing, and system tray menus.',
  },
  {
    name: 'ContentDialog, TeachingTip & Flyouts',
    detail: 'Layer-anchored popovers, focus-trapped modals, and AutoSuggestBox.',
  },
  {
    name: 'TreeView, DataGrid & ColorPicker',
    detail: 'Virtualized lists, HSV color wheels, NumberBox, and Markdown tables.',
  },
] as const;

const MOBILE_SYSTEMS = [
  {
    name: 'Multi-touch Gesture Arena & Velocity Fling',
    detail: 'Disambiguates tap, pan, pinch, and scroll drags with iOS spring & bounce physics.',
  },
  {
    name: 'Full IME Composition & Virtual Keyboard Insets',
    detail: 'CJK marked-text underline, selection handles, and automatic safe-area scroll padding.',
  },
  {
    name: 'CupertinoSliverNavigationBar & Modal Sheets',
    detail: 'Large-title collapse on scroll, swipe-to-pop page transitions, and bottom sheets.',
  },
  {
    name: 'CupertinoDatePicker, ActionSheet & ContextMenu',
    detail: '3D cylindrical wheel pickers, segmented controls, and haptics hooks.',
  },
] as const;

export function DesktopMobileParity() {
  return (
    <section
      className="border-y px-6 py-20 sm:py-24"
      style={{
        borderColor: 'var(--site-hairline)',
        background: 'var(--site-surface)',
      }}
    >
      <div className="mx-auto max-w-[1160px]">
        {/* Header */}
        <div className="flex flex-col justify-between gap-4 md:flex-row md:items-end">
          <div>
            <p
              className="font-mono text-[12px] font-semibold uppercase tracking-[0.12em]"
              style={{ color: 'var(--site-accent)' }}
            >
              02 — Desktop &amp; Mobile First-Class
            </p>
            <h2
              className="mt-2 text-[clamp(1.85rem,3.6vw,2.75rem)] leading-[1.08] font-semibold tracking-[-0.03em]"
              style={{ color: 'var(--site-ink)' }}
            >
              Most Rust UI frameworks stop at desktop.
              <br className="hidden sm:inline" /> Waft ships complete mobile ergonomics too.
            </h2>
          </div>
          <p className="max-w-[440px] text-[15px] leading-[1.6]" style={{ color: 'var(--site-muted)' }}>
            Mobile UI is much harder than drawing buttons in a window: it demands gesture disambiguation, IME composition,
            keyboard safe-area insets, physics scrolling, and sheet navigation. Waft builds all of them into the core
            pipeline.
          </p>
        </div>

        {/* Two side-by-side editorial specification cards */}
        <div className="mt-10 grid grid-cols-1 gap-6 lg:grid-cols-12">
          {/* Desktop Card (6 cols) */}
          <div
            className="flex flex-col justify-between rounded-2xl border p-6 sm:p-7 lg:col-span-6"
            style={{
              background: 'var(--site-card)',
              borderColor: 'var(--site-hairline-strong)',
            }}
          >
            <div>
              <div className="flex items-center justify-between">
                <div className="inline-flex items-center gap-2">
                  <span
                    className="flex size-8 items-center justify-center rounded-lg"
                    style={{ background: 'rgba(0, 95, 184, 0.1)', color: 'var(--site-cobalt)' }}
                  >
                    <Monitor className="size-4" />
                  </span>
                  <div>
                    <div className="text-[16px] font-semibold" style={{ color: 'var(--site-ink)' }}>
                      Desktop &amp; Web Kits
                    </div>
                    <div className="font-mono text-[11px]" style={{ color: 'var(--site-subtle)' }}>
                      waft-winui — waft-material — waft-markdown
                    </div>
                  </div>
                </div>
                <a
                  href={galleryUrls.winui}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-1 rounded-full border px-3 py-1 text-[12px] font-medium transition hover:opacity-80"
                  style={{
                    borderColor: 'var(--site-hairline-strong)',
                    color: 'var(--site-ink)',
                  }}
                >
                  WinUI Gallery
                  <ExternalLink className="size-3" />
                </a>
              </div>

              <div className="mt-6 divide-y" style={{ borderColor: 'var(--site-hairline)' }}>
                {DESKTOP_SYSTEMS.map((item) => (
                  <div key={item.name} className="py-3 first:pt-0 last:pb-0">
                    <div className="text-[14px] font-semibold" style={{ color: 'var(--site-ink)' }}>
                      {item.name}
                    </div>
                    <div className="mt-0.5 text-[13px] leading-[1.5]" style={{ color: 'var(--site-muted)' }}>
                      {item.detail}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <div
              className="mt-6 flex flex-wrap items-center justify-between gap-2 rounded-xl border px-3.5 py-2.5 font-mono text-[11.5px]"
              style={{
                background: 'var(--site-surface)',
                borderColor: 'var(--site-hairline)',
                color: 'var(--site-muted)',
              }}
            >
              <span>Targets: macOS (.app/.dmg) — Windows (.exe/.msi) — Linux (.deb)</span>
              <Link
                href="/docs/shortcuts"
                className="inline-flex items-center gap-0.5 font-sans text-[12px] font-semibold"
                style={{ color: 'var(--site-ink)' }}
              >
                Shortcuts guide <ArrowUpRight className="size-3.5" />
              </Link>
            </div>
          </div>

          {/* Mobile Card (6 cols) */}
          <div
            className="flex flex-col justify-between rounded-2xl border p-6 sm:p-7 lg:col-span-6"
            style={{
              background: 'var(--site-card)',
              borderColor: 'var(--site-hairline-strong)',
            }}
          >
            <div>
              <div className="flex items-center justify-between">
                <div className="inline-flex items-center gap-2">
                  <span
                    className="flex size-8 items-center justify-center rounded-lg"
                    style={{ background: 'rgba(200, 75, 17, 0.1)', color: 'var(--site-accent)' }}
                  >
                    <Smartphone className="size-4" />
                  </span>
                  <div>
                    <div className="text-[16px] font-semibold" style={{ color: 'var(--site-ink)' }}>
                      Mobile &amp; Touch Kits
                    </div>
                    <div className="font-mono text-[11px]" style={{ color: 'var(--site-subtle)' }}>
                      waft-cupertino — waft-material — waft-gestures
                    </div>
                  </div>
                </div>
                <a
                  href={galleryUrls.cupertino}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-1 rounded-full border px-3 py-1 text-[12px] font-medium transition hover:opacity-80"
                  style={{
                    borderColor: 'var(--site-hairline-strong)',
                    color: 'var(--site-ink)',
                  }}
                >
                  Cupertino Gallery
                  <ExternalLink className="size-3" />
                </a>
              </div>

              <div className="mt-6 divide-y" style={{ borderColor: 'var(--site-hairline)' }}>
                {MOBILE_SYSTEMS.map((item) => (
                  <div key={item.name} className="py-3 first:pt-0 last:pb-0">
                    <div className="text-[14px] font-semibold" style={{ color: 'var(--site-ink)' }}>
                      {item.name}
                    </div>
                    <div className="mt-0.5 text-[13px] leading-[1.5]" style={{ color: 'var(--site-muted)' }}>
                      {item.detail}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <div
              className="mt-6 flex flex-wrap items-center justify-between gap-2 rounded-xl border px-3.5 py-2.5 font-mono text-[11.5px]"
              style={{
                background: 'var(--site-surface)',
                borderColor: 'var(--site-hairline)',
                color: 'var(--site-muted)',
              }}
            >
              <span>Targets: iOS (.app/.ipa) — Android (.apk/.aab) — WASI (.wapk)</span>
              <Link
                href="/docs/text-input"
                className="inline-flex items-center gap-0.5 font-sans text-[12px] font-semibold"
                style={{ color: 'var(--site-ink)' }}
              >
                IME &amp; Text guide <ArrowUpRight className="size-3.5" />
              </Link>
            </div>
          </div>
        </div>

        {/* Performance & Footprint Strip (Pure Rust + valo + wgpu) */}
        <div
          className="mt-6 rounded-2xl border p-6 sm:p-7"
          style={{
            background: 'var(--site-card)',
            borderColor: 'var(--site-hairline-strong)',
          }}
        >
          <div className="grid grid-cols-1 gap-6 lg:grid-cols-12 lg:items-center">
            <div className="lg:col-span-5">
              <div className="inline-flex items-center gap-2 font-mono text-[11.5px] font-semibold uppercase tracking-[0.08em]" style={{ color: 'var(--site-accent)' }}>
                <Gauge className="size-3.5" />
                Pure Rust + valo + wgpu
              </div>
              <h3 className="mt-1.5 text-[20px] leading-[1.2] font-semibold tracking-[-0.02em]" style={{ color: 'var(--site-ink)' }}>
                Extreme performance. Minimal memory and CPU.
              </h3>
              <p className="mt-2 text-[13.5px] leading-[1.55]" style={{ color: 'var(--site-muted)' }}>
                No Chromium runtime, no JS or Dart VM bridge, and no garbage collector pauses. Display lists compile
                straight into <code className="font-mono">valo</code> GPU passes on Metal, Vulkan, DX12, and WebGPU.
              </p>
            </div>

            <div className="grid grid-cols-1 gap-3 sm:grid-cols-3 lg:col-span-7">
              <div
                className="rounded-xl border p-4"
                style={{ background: 'var(--site-surface)', borderColor: 'var(--site-hairline)' }}
              >
                <div className="flex items-center justify-between">
                  <span className="font-mono text-[11px] font-semibold uppercase" style={{ color: 'var(--site-subtle)' }}>
                    Memory Overhead
                  </span>
                  <Cpu className="size-3.5" style={{ color: 'var(--site-cobalt)' }} />
                </div>
                <div className="mt-2 font-mono text-[18px] font-semibold" style={{ color: 'var(--site-ink)' }}>
                  Arena-allocated
                </div>
                <p className="mt-1 text-[12px] leading-[1.45]" style={{ color: 'var(--site-muted)' }}>
                  Generational handles in a single Rust arena instead of a 150 MB browser engine per app.
                </p>
              </div>

              <div
                className="rounded-xl border p-4"
                style={{ background: 'var(--site-surface)', borderColor: 'var(--site-hairline)' }}
              >
                <div className="flex items-center justify-between">
                  <span className="font-mono text-[11px] font-semibold uppercase" style={{ color: 'var(--site-subtle)' }}>
                    Frame Pipeline
                  </span>
                  <Gauge className="size-3.5" style={{ color: 'var(--site-accent)' }} />
                </div>
                <div className="mt-2 font-mono text-[18px] font-semibold" style={{ color: 'var(--site-ink)' }}>
                  Zero GC Pauses
                </div>
                <p className="mt-1 text-[12px] leading-[1.45]" style={{ color: 'var(--site-muted)' }}>
                  Deterministic layout and GPU submission at 120 Hz with idle CPU at 0% between input events.
                </p>
              </div>

              <div
                className="rounded-xl border p-4"
                style={{ background: 'var(--site-surface)', borderColor: 'var(--site-hairline)' }}
              >
                <div className="flex items-center justify-between">
                  <span className="font-mono text-[11px] font-semibold uppercase" style={{ color: 'var(--site-subtle)' }}>
                    Safety Contract
                  </span>
                  <ShieldCheck className="size-3.5 text-emerald-600" />
                </div>
                <div className="mt-2 font-mono text-[18px] font-semibold" style={{ color: 'var(--site-ink)' }}>
                  0 unsafe blocks
                </div>
                <p className="mt-1 text-[12px] leading-[1.45]" style={{ color: 'var(--site-muted)' }}>
                  <code className="font-mono">#![forbid(unsafe_code)]</code> enforced across the framework, kits, and
                  layout crates.
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
