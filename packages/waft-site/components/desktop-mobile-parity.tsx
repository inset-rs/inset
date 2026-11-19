import { ExternalLink } from 'lucide-react';
import { galleryUrls } from '@/lib/shared';
import { RustCodeBlock } from './code-highlight';

type KitCard = {
  crateName: string;
  title: string;
  galleryUrl: string;
  galleryLabel: string;
  summary: string;
  code: string;
};

const KIT_CARDS: KitCard[] = [
  {
    crateName: 'waft-winui',
    title: 'WinUI 3',
    galleryUrl: galleryUrls.winui,
    galleryLabel: 'WinUI Gallery',
    summary:
      'NavigationView, TabView, ContentDialog, MenuBar, TeachingTip, and hardware keyboard accelerators for desktop and web workflows.',
    code: `NavigationView::new()
    .pane_title("Workspace")
    .item("Files", FileTree::new())
    .footer(Button::accent("Deploy"))`,
  },
  {
    crateName: 'waft-cupertino',
    title: 'Cupertino',
    galleryUrl: galleryUrls.cupertino,
    galleryLabel: 'Cupertino Gallery',
    summary:
      'Multi-touch gesture arena, iOS spring scrolling, CupertinoSliverNavigationBar, modal sheets, CJK IME composition, and keyboard insets.',
    code: `CupertinoPageScaffold::new()
    .bar(CupertinoNavBar::new("Notes"))
    .child(CupertinoList::new(rows))
    .into_widget()`,
  },
  {
    crateName: 'waft-material',
    title: 'Material 3',
    galleryUrl: galleryUrls.material,
    galleryLabel: 'Material Gallery',
    summary:
      'Adaptive ThreePaneNavigator, Scaffold, AppBar, NavigationBar, NavigationDrawer, Autocomplete, and Material 3 tonal palettes.',
    code: `Scaffold::new()
    .app_bar(AppBar::new("Inbox"))
    .drawer(NavDrawer::new(folders))
    .body(ThreePane::new(panes))`,
  },
];

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
        <div className="grid grid-cols-1 gap-6 lg:grid-cols-12 lg:items-end">
          <div className="lg:col-span-7">
            <h2
              className="text-[32px] leading-[1.08] font-semibold tracking-[-0.03em] sm:text-[42px]"
              style={{ color: 'var(--site-ink)' }}
            >
              Most Rust UI stops at desktop.
              <br />
              Waft ships complete mobile ergonomics too.
            </h2>
          </div>
          <p
            className="text-[15.5px] leading-[1.6] lg:col-span-5"
            style={{ color: 'var(--site-muted)' }}
          >
            Mobile UI demands gesture disambiguation, IME composition, keyboard safe-area insets, physics scrolling,
            and sheet navigation. Waft builds all of them into the core pipeline and ships three complete widget kits.
          </p>
        </div>

        {/* 3 clean cards: WinUI, Cupertino, Material with live gallery links & highlighted code */}
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
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <code
                      className="font-mono text-[12.5px] font-semibold"
                      style={{ color: 'var(--site-accent)' }}
                    >
                      {kit.crateName}
                    </code>
                    <h3
                      className="mt-1 text-[20px] font-semibold tracking-[-0.015em]"
                      style={{ color: 'var(--site-ink)' }}
                    >
                      {kit.title}
                    </h3>
                  </div>
                  <a
                    href={kit.galleryUrl}
                    target="_blank"
                    rel="noreferrer"
                    className="inline-flex shrink-0 items-center gap-1 rounded-full border px-3 py-1 text-[12px] font-medium transition hover:opacity-80"
                    style={{
                      borderColor: 'var(--site-hairline-strong)',
                      color: 'var(--site-ink)',
                    }}
                  >
                    {kit.galleryLabel}
                    <ExternalLink className="size-3" />
                  </a>
                </div>

                <p
                  className="mt-3 text-[13.5px] leading-[1.55]"
                  style={{ color: 'var(--site-muted)' }}
                >
                  {kit.summary}
                </p>
              </div>

              <div className="mt-5">
                <RustCodeBlock code={kit.code} />
              </div>
            </div>
          ))}
        </div>

        {/* Clean 3-column performance & safety summary without nested boxes */}
        <div
          className="mt-10 grid grid-cols-1 gap-8 border-t pt-8 sm:grid-cols-3"
          style={{ borderColor: 'var(--site-hairline-strong)' }}
        >
          <div>
            <h3 className="text-[16px] font-semibold" style={{ color: 'var(--site-ink)' }}>
              Arena-allocated, no browser engine
            </h3>
            <p className="mt-1.5 text-[13.5px] leading-[1.55]" style={{ color: 'var(--site-muted)' }}>
              Generational handles in a single Rust arena instead of bundling a 150 MB Chromium runtime or language VM
              with every app.
            </p>
          </div>

          <div>
            <h3 className="text-[16px] font-semibold" style={{ color: 'var(--site-ink)' }}>
              Direct valo + wgpu rendering
            </h3>
            <p className="mt-1.5 text-[13.5px] leading-[1.55]" style={{ color: 'var(--site-muted)' }}>
              Display lists compile straight into <code className="font-mono">valo</code> GPU passes on Metal, Vulkan,
              DirectX 12, and WebGPU with zero garbage-collection pauses.
            </p>
          </div>

          <div>
            <h3 className="text-[16px] font-semibold" style={{ color: 'var(--site-ink)' }}>
              #![forbid(unsafe_code)] across the stack
            </h3>
            <p className="mt-1.5 text-[13.5px] leading-[1.55]" style={{ color: 'var(--site-muted)' }}>
              Every widget kit, layout algorithm, gesture recognizer, and state container is 100% safe Rust.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
