import Link from 'next/link';
import { ArrowUpRight, Box, Cpu, ExternalLink, Layers, Sparkles } from 'lucide-react';

interface CrateLink {
  name: string;
  href?: string;
}

interface LayerSpec {
  step: string;
  label: string;
  title: string;
  summary: string;
  docsHref: string;
  docsLabel: string;
  icon: 'kits' | 'engine' | 'embedder' | 'gpu';
  crates: CrateLink[];
}

const LAYERS: LayerSpec[] = [
  {
    step: '01',
    label: 'KITS',
    title: 'Design System Kits',
    summary:
      'Ordinary Cargo crates built on Waft widgets. Use a full kit, mix controls across kits, or fork one crate for your own design system.',
    docsHref: '/docs/kits',
    docsLabel: 'UI kits guide',
    icon: 'kits',
    crates: [
      { name: 'waft-winui', href: 'https://docs.rs/waft-winui/latest/waft_winui/' },
      { name: 'waft-cupertino', href: 'https://docs.rs/waft-cupertino/latest/waft_cupertino/' },
      { name: 'waft-material', href: 'https://docs.rs/waft-material/latest/waft_material/' },
      { name: 'waft-markdown', href: 'https://docs.rs/waft-markdown/latest/waft_markdown/' },
    ],
  },
  {
    step: '02',
    label: 'ENGINE',
    title: 'Flutter-Model Widget & Layout Pipeline',
    summary:
      'Widgets, elements, box and sliver layout, multi-touch gesture arena, IME text input, and generational arena handles without borrow-checker gymnastics.',
    docsHref: '/docs/architecture',
    docsLabel: 'Architecture guide',
    icon: 'engine',
    crates: [
      { name: 'waft-widgets', href: 'https://docs.rs/waft-widgets/latest/waft_widgets/' },
      { name: 'waft-rendering', href: 'https://docs.rs/waft-rendering/latest/waft_rendering/' },
      { name: 'waft-gestures', href: 'https://docs.rs/waft-gestures/latest/waft_gestures/' },
      { name: 'waft-foundation', href: 'https://docs.rs/waft-foundation/latest/waft_foundation/' },
    ],
  },
  {
    step: '03',
    label: 'BOUNDARY',
    title: 'Embedder Trait Boundary',
    summary:
      'Platform, View, and host services expressed as pure Rust traits in waft-embedder. Swap hosts freely or embed Waft inside any engine or WASI sandbox.',
    docsHref: '/docs/embedder',
    docsLabel: 'Embedder guide',
    icon: 'embedder',
    crates: [
      { name: 'waft-embedder', href: 'https://docs.rs/waft-embedder/latest/waft_embedder/' },
      { name: 'waft-shell', href: 'https://docs.rs/waft-shell/latest/waft_shell/' },
      { name: 'waft-test', href: 'https://docs.rs/waft-test/latest/waft_test/' },
    ],
  },
  {
    step: '04',
    label: 'GPU & HOSTS',
    title: 'Pure Rust GPU Compositor (valo + wgpu)',
    summary:
      'Direct Metal, Vulkan, DX12, and WebGPU rendering with zero C++ build scripts across desktop, mobile, browser, and WASI targets.',
    docsHref: '/docs/reference',
    docsLabel: 'All crates',
    icon: 'gpu',
    crates: [
      { name: 'macOS / iOS (Metal)' },
      { name: 'Windows (DX12)' },
      { name: 'Linux / Android (Vulkan)' },
      { name: 'Web / WASI (WebGPU)' },
    ],
  },
];

function LayerIcon({ kind }: { kind: LayerSpec['icon'] }) {
  if (kind === 'kits') return <Sparkles className="size-4" />;
  if (kind === 'engine') return <Layers className="size-4" />;
  if (kind === 'embedder') return <Box className="size-4" />;
  return <Cpu className="size-4" />;
}

export function ArchitectureExplorer() {
  return (
    <section className="px-4 py-16 sm:px-6 sm:py-20">
      <div className="mx-auto max-w-[1160px]">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <h2
            className="text-[32px] leading-[1.08] font-semibold tracking-[-0.03em] sm:text-[42px]"
            style={{ color: 'var(--site-ink)' }}
          >
            Ordinary Cargo crates.
            <br />
            Clean trait boundary at the bottom.
          </h2>

          <Link
            href="/docs/reference"
            className="inline-flex items-center gap-1.5 rounded-full border px-4 py-2 text-[13px] font-medium transition hover:opacity-80"
            style={{
              background: 'var(--site-card)',
              borderColor: 'var(--site-hairline-strong)',
              color: 'var(--site-ink)',
            }}
          >
            Browse all crates on docs.rs
            <ArrowUpRight className="size-3.5" />
          </Link>
        </div>

        {/* Full-width 4-layer stack map — every layer visible at once, zero code walls */}
        <div className="mt-10 flex flex-col gap-3.5">
          {LAYERS.map((layer) => (
            <div
              key={layer.step}
              className="grid grid-cols-1 items-center gap-5 rounded-2xl border p-5 sm:p-6 lg:grid-cols-12"
              style={{
                background: 'var(--site-card)',
                borderColor: 'var(--site-hairline-strong)',
              }}
            >
              {/* Left 5 cols: Layer identity + 1-line role */}
              <div className="flex items-start gap-3.5 lg:col-span-5">
                <span
                  className="mt-0.5 inline-flex size-9 shrink-0 items-center justify-center rounded-xl"
                  style={{
                    background: 'var(--site-accent-soft)',
                    color: 'var(--site-accent)',
                  }}
                >
                  <LayerIcon kind={layer.icon} />
                </span>
                <div>
                  <div className="flex items-center gap-2 font-mono text-[11px] font-semibold tracking-[0.06em]">
                    <span style={{ color: 'var(--site-accent)' }}>{layer.step}</span>
                    <span style={{ color: 'var(--site-subtle)' }}>{layer.label}</span>
                  </div>
                  <h3
                    className="mt-0.5 text-[17px] font-semibold tracking-[-0.015em]"
                    style={{ color: 'var(--site-ink)' }}
                  >
                    {layer.title}
                  </h3>
                  <p className="mt-1 text-[13.5px] leading-[1.5]" style={{ color: 'var(--site-muted)' }}>
                    {layer.summary}
                  </p>
                </div>
              </div>

              {/* Right 7 cols: Clickable crate pills (linking directly to docs.rs) + guide link */}
              <div className="flex flex-col justify-between gap-3 sm:flex-row sm:items-center lg:col-span-7 lg:pl-4">
                <div className="flex flex-wrap gap-2">
                  {layer.crates.map((crate) =>
                    crate.href ? (
                      <a
                        key={crate.name}
                        href={crate.href}
                        target="_blank"
                        rel="noreferrer"
                        className="inline-flex items-center gap-1.5 rounded-lg border px-3 py-1.5 font-mono text-[12px] font-medium transition hover:border-stone-400"
                        style={{
                          background: 'var(--site-surface)',
                          borderColor: 'var(--site-hairline-strong)',
                          color: 'var(--site-ink)',
                        }}
                      >
                        <span>{crate.name}</span>
                        <ExternalLink className="size-3" style={{ color: 'var(--site-subtle)' }} />
                      </a>
                    ) : (
                      <span
                        key={crate.name}
                        className="inline-flex items-center rounded-lg border px-3 py-1.5 font-mono text-[12px]"
                        style={{
                          background: 'var(--site-surface)',
                          borderColor: 'var(--site-hairline)',
                          color: 'var(--site-muted)',
                        }}
                      >
                        {crate.name}
                      </span>
                    ),
                  )}
                </div>

                <Link
                  href={layer.docsHref}
                  className="inline-flex shrink-0 items-center gap-1 text-[13px] font-medium transition hover:opacity-75"
                  style={{ color: 'var(--site-accent)' }}
                >
                  {layer.docsLabel}
                  <ArrowUpRight className="size-3.5" />
                </Link>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
