import { ExternalLink, Layers, Monitor, Smartphone } from 'lucide-react';
import { galleryUrls } from '@/lib/shared';

type KitCard = {
  id: 'winui' | 'cupertino' | 'material';
  crateName: string;
  title: string;
  galleryUrl: string;
  galleryLabel: string;
  docsRsUrl: string;
  chips: string[];
  summary: string;
};

const KIT_CARDS: KitCard[] = [
  {
    id: 'winui',
    crateName: 'waft-winui',
    title: 'WinUI 3',
    galleryUrl: galleryUrls.winui,
    galleryLabel: 'WinUI Gallery',
    docsRsUrl: 'https://docs.rs/waft-winui/latest/waft_winui/',
    chips: ['NavigationView', 'TabView', 'MenuBar'],
    summary:
      'Fluent controls, ContentDialog, TeachingTip, and hardware keyboard accelerators built for dense desktop and web workflows.',
  },
  {
    id: 'cupertino',
    crateName: 'waft-cupertino',
    title: 'Cupertino',
    galleryUrl: galleryUrls.cupertino,
    galleryLabel: 'Cupertino Gallery',
    docsRsUrl: 'https://docs.rs/waft-cupertino/latest/waft_cupertino/',
    chips: ['SliverNavBar', 'Spring Scroll', 'CJK IME'],
    summary:
      'Multi-touch gesture arena, iOS spring physics, modal sheets, software keyboard insets, and native text selection.',
  },
  {
    id: 'material',
    crateName: 'waft-material',
    title: 'Material 3',
    galleryUrl: galleryUrls.material,
    galleryLabel: 'Material Gallery',
    docsRsUrl: 'https://docs.rs/waft-material/latest/waft_material/',
    chips: ['ThreePane', 'Scaffold', 'Tonal M3'],
    summary:
      'Adaptive ThreePaneNavigator, AppBar, NavigationBar, NavigationDrawer, Autocomplete, and dynamic Material 3 color schemes.',
  },
];

function KitIcon({ id }: { id: KitCard['id'] }) {
  if (id === 'winui') return <Monitor className="size-4" />;
  if (id === 'cupertino') return <Smartphone className="size-4" />;
  return <Layers className="size-4" />;
}

export function DesktopMobileParity() {
  return (
    <section
      className="border-y px-4 py-16 sm:px-6 sm:py-20"
      style={{
        background: 'var(--site-surface)',
        borderColor: 'var(--site-hairline)',
      }}
    >
      <div className="mx-auto max-w-[1160px]">
        <h2
          className="text-[32px] leading-[1.08] font-semibold tracking-[-0.03em] sm:text-[42px]"
          style={{ color: 'var(--site-ink)' }}
        >
          Most Rust UI stops at desktop.
          <br />
          Waft ships complete mobile ergonomics too.
        </h2>

        {/* 3 clean kit showcase cards — zero repetitive code boxes */}
        <div className="mt-10 grid grid-cols-1 gap-5 lg:grid-cols-3">
          {KIT_CARDS.map((kit) => (
            <div
              key={kit.crateName}
              className="flex flex-col justify-between rounded-2xl border p-6"
              style={{
                background: 'var(--site-card)',
                borderColor: 'var(--site-hairline-strong)',
              }}
            >
              <div>
                {/* Compact visual header strip */}
                <div
                  className="flex items-center justify-between gap-2 rounded-xl border px-3.5 py-3"
                  style={{
                    background: 'var(--site-surface)',
                    borderColor: 'var(--site-hairline)',
                  }}
                >
                  <div className="flex items-center gap-2.5">
                    <span
                      className="inline-flex size-7 items-center justify-center rounded-lg"
                      style={{
                        background: 'var(--site-accent-soft)',
                        color: 'var(--site-accent)',
                      }}
                    >
                      <KitIcon id={kit.id} />
                    </span>
                    <h3
                      className="text-[17px] font-semibold tracking-[-0.015em]"
                      style={{ color: 'var(--site-ink)' }}
                    >
                      {kit.title}
                    </h3>
                  </div>

                  <span
                    className="rounded-md border px-2 py-0.5 font-mono text-[11px]"
                    style={{
                      background: 'var(--site-card)',
                      borderColor: 'var(--site-hairline-strong)',
                      color: 'var(--site-muted)',
                    }}
                  >
                    {kit.crateName}
                  </span>
                </div>

                {/* Key controls chip row */}
                <div className="mt-4 flex flex-wrap gap-1.5">
                  {kit.chips.map((chip) => (
                    <span
                      key={chip}
                      className="rounded-md border px-2.5 py-1 font-mono text-[11px]"
                      style={{
                        background: 'var(--site-surface)',
                        borderColor: 'var(--site-hairline)',
                        color: 'var(--site-ink)',
                      }}
                    >
                      {chip}
                    </span>
                  ))}
                </div>

                <p className="mt-4 text-[14px] leading-[1.6]" style={{ color: 'var(--site-muted)' }}>
                  {kit.summary}
                </p>
              </div>

              <div
                className="mt-6 flex items-center justify-between gap-3 border-t pt-4 text-[13px] font-medium"
                style={{ borderColor: 'var(--site-hairline)' }}
              >
                <a
                  href={kit.galleryUrl}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-1 transition hover:opacity-75"
                  style={{ color: 'var(--site-accent)' }}
                >
                  {kit.galleryLabel}
                  <ExternalLink className="size-3.5" />
                </a>

                <a
                  href={kit.docsRsUrl}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-1 font-mono text-[12px] transition hover:opacity-75"
                  style={{ color: 'var(--site-muted)' }}
                >
                  docs.rs
                  <ExternalLink className="size-3" />
                </a>
              </div>
            </div>
          ))}
        </div>

        {/* Clean 3-column platform facts */}
        <div
          className="mt-8 grid grid-cols-1 gap-6 border-t pt-7 md:grid-cols-3"
          style={{ borderColor: 'var(--site-hairline-strong)' }}
        >
          <div>
            <div className="text-[14px] font-semibold" style={{ color: 'var(--site-ink)' }}>
              True multi-touch &amp; CJK IME
            </div>
            <p className="mt-1 text-[13.5px] leading-[1.55]" style={{ color: 'var(--site-muted)' }}>
              Full gesture disambiguation, spring bounce physics, software keyboard insets, and composition underlines
              on iOS, Android, macOS, and Windows.
            </p>
          </div>
          <div>
            <div className="text-[14px] font-semibold" style={{ color: 'var(--site-ink)' }}>
              Pure Rust GPU pipeline
            </div>
            <p className="mt-1 text-[13.5px] leading-[1.55]" style={{ color: 'var(--site-muted)' }}>
              Rendered directly by <code className="font-mono">valo</code> and <code className="font-mono">wgpu</code>{' '}
              on Metal, Vulkan, DX12, and WebGPU — zero Skia, zero C++ build scripts.
            </p>
          </div>
          <div>
            <div className="text-[14px] font-semibold" style={{ color: 'var(--site-ink)' }}>
              One codebase across 7 targets
            </div>
            <p className="mt-1 text-[13.5px] leading-[1.55]" style={{ color: 'var(--site-muted)' }}>
              Compile the same Cargo library crate for macOS, Windows, Linux, iOS, Android, WebGPU browsers, and WaOS
              WASI hosts without rewriting UI code.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
