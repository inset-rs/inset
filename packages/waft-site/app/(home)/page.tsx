import Link from 'next/link';
import { ArrowRight, ArrowUpRight, ExternalLink } from 'lucide-react';
import { ArchitectureExplorer } from '@/components/architecture-explorer';
import { RustCodeBlock, ShellCodeBlock } from '@/components/code-highlight';
import { DesktopMobileParity } from '@/components/desktop-mobile-parity';
import { HeroStudio } from '@/components/hero-studio';
import {
  description,
  edgedUrl,
  galleryUrls,
  githubUrl,
  headline,
  waosUrl,
} from '@/lib/shared';

const HEADLESS_TEST_CODE = `#[test]
fn audio_pipeline_toggles_cleanly() {
    let mut app = AppCell::new();
    let studio = StudioSession::new(&mut app);

    studio.toggle_low_latency(&mut app);
    assert!(studio.is_exclusive_buffer(&app));

    studio.export_preset(&mut app, "master.preset");
    assert_eq!(studio.exported_count(&app), 1);
}`;

const CLI_WORKFLOW_CODE = `# Install the CLI and scaffold a project
cargo install waft-cli
cargo waft new notes

# Run on desktop, mobile simulator, or browser
cargo waft run
cargo waft run -d ios
cargo waft run -d android
cargo waft run -d web

# Package signed native bundles and installers
cargo waft build dmg   # msi, deb, ipa, apk, web, wasm`;

export default function HomePage() {
  return (
    <main className="flex flex-1 flex-col">
      {/* 1. Editorial Hero + Live UI Kit Studio */}
      <section className="site-grid-paper relative overflow-hidden border-b px-4 pt-10 pb-16 sm:px-6 sm:pt-14 sm:pb-20" style={{ borderColor: 'var(--site-hairline)' }}>
        <div className="mx-auto max-w-[1160px]">
          <div className="grid grid-cols-1 gap-6 lg:grid-cols-12 lg:items-end">
            <div className="lg:col-span-8">
              <h1
                className="text-[40px] leading-[1.03] font-semibold tracking-[-0.035em] sm:text-[58px] lg:text-[64px]"
                style={{ color: 'var(--site-ink)' }}
              >
                {headline[0]}
                <br />
                <span style={{ color: 'var(--site-muted)' }}>{headline[1]}</span>
              </h1>
            </div>

            <div className="lg:col-span-4 lg:pb-1">
              <p
                className="text-[16px] leading-[1.6]"
                style={{ color: 'var(--site-muted)' }}
              >
                {description}
              </p>

              {/* Developer CTAs */}
              <div className="mt-5 flex flex-wrap items-center gap-2.5">
                <Link
                  href="/docs/start"
                  className="inline-flex h-10 items-center gap-2 rounded-xl px-4 text-[13.5px] font-medium text-white shadow-xs transition hover:opacity-90"
                  style={{ background: 'var(--site-ink)' }}
                >
                  <span>Get Started</span>
                  <ArrowRight className="size-4" />
                </Link>
                <Link
                  href="/docs/architecture"
                  className="inline-flex h-10 items-center gap-1.5 rounded-xl border px-4 text-[13.5px] font-medium transition hover:opacity-80"
                  style={{
                    background: 'var(--site-card)',
                    borderColor: 'var(--site-hairline-strong)',
                    color: 'var(--site-ink)',
                  }}
                >
                  <span>Documentation</span>
                </Link>
                <a
                  href={githubUrl}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex h-10 items-center gap-1.5 rounded-xl border px-4 text-[13.5px] font-medium transition hover:opacity-80"
                  style={{
                    background: 'var(--site-card)',
                    borderColor: 'var(--site-hairline-strong)',
                    color: 'var(--site-ink)',
                  }}
                >
                  <span>GitHub</span>
                  <ExternalLink className="size-3.5" />
                </a>
              </div>
            </div>
          </div>

          {/* Interactive Kit Studio */}
          <div className="mt-10">
            <HeroStudio />
          </div>

          {/* Built with Waft & Live Galleries bar */}
          <div
            className="mt-8 flex flex-wrap items-center justify-between gap-4 rounded-2xl border px-5 py-3.5 text-[13px]"
            style={{
              background: 'var(--site-card)',
              borderColor: 'var(--site-hairline-strong)',
            }}
          >
            <span className="font-medium" style={{ color: 'var(--site-muted)' }}>
              Live WebGPU galleries &amp; apps built with Waft
            </span>
            <div className="flex flex-wrap items-center gap-5">
              <a
                href={galleryUrls.winui}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1 font-medium transition hover:opacity-75"
                style={{ color: 'var(--site-ink)' }}
              >
                WinUI Gallery
                <ArrowUpRight className="size-3.5" style={{ color: 'var(--site-subtle)' }} />
              </a>
              <a
                href={galleryUrls.cupertino}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1 font-medium transition hover:opacity-75"
                style={{ color: 'var(--site-ink)' }}
              >
                Cupertino Gallery
                <ArrowUpRight className="size-3.5" style={{ color: 'var(--site-subtle)' }} />
              </a>
              <a
                href={galleryUrls.material}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1 font-medium transition hover:opacity-75"
                style={{ color: 'var(--site-ink)' }}
              >
                Material Gallery
                <ArrowUpRight className="size-3.5" style={{ color: 'var(--site-subtle)' }} />
              </a>
              <a
                href={edgedUrl}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1 font-medium transition hover:opacity-75"
                style={{ color: 'var(--site-ink)' }}
              >
                Edged 2
                <ArrowUpRight className="size-3.5" style={{ color: 'var(--site-subtle)' }} />
              </a>
              <a
                href={waosUrl}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1 font-medium transition hover:opacity-75"
                style={{ color: 'var(--site-ink)' }}
              >
                WaOS Built-in Apps
                <ArrowUpRight className="size-3.5" style={{ color: 'var(--site-subtle)' }} />
              </a>
            </div>
          </div>
        </div>
      </section>

      {/* 2. Modular Rust Architecture & Clear Embedder Interface */}
      <ArchitectureExplorer />

      {/* 3. Desktop & Mobile First-Class Kits + valo/wgpu Performance */}
      <DesktopMobileParity />

      {/* 4. Headless Testing & cargo waft CLI */}
      <section className="px-4 py-16 sm:px-6 sm:py-20">
        <div className="mx-auto max-w-[1160px]">
          <div className="grid grid-cols-1 gap-6 lg:grid-cols-12 lg:items-end">
            <div className="lg:col-span-7">
              <h2
                className="text-[32px] leading-[1.08] font-semibold tracking-[-0.03em] sm:text-[42px]"
                style={{ color: 'var(--site-ink)' }}
              >
                Verify in milliseconds without a display.
                <br />
                Package everywhere with cargo waft.
              </h2>
            </div>
            <p
              className="text-[15.5px] leading-[1.6] lg:col-span-5"
              style={{ color: 'var(--site-muted)' }}
            >
              Run widget and state tests in standard <code className="font-mono">cargo test</code> with no window
              server, and build signed bundles for every desktop, mobile, web, and WASI target from one CLI.
            </p>
          </div>

          <div className="mt-10 grid grid-cols-1 gap-6 lg:grid-cols-12">
            {/* Left 6 cols: Headless AppCell & waft-test */}
            <div
              className="flex flex-col justify-between rounded-2xl border p-6 sm:p-7 lg:col-span-6"
              style={{
                background: 'var(--site-card)',
                borderColor: 'var(--site-hairline-strong)',
              }}
            >
              <div>
                <div className="flex items-center justify-between gap-2">
                  <h3 className="text-[18px] font-semibold tracking-[-0.015em]" style={{ color: 'var(--site-ink)' }}>
                    Headless testing with AppCell
                  </h3>
                  <Link
                    href="/docs/testing"
                    className="inline-flex items-center gap-1 text-[13px] font-medium transition hover:opacity-75"
                    style={{ color: 'var(--site-ink)' }}
                  >
                    Testing guide <ArrowUpRight className="size-3.5" />
                  </Link>
                </div>

                <p className="mt-2.5 text-[14px] leading-[1.55]" style={{ color: 'var(--site-muted)' }}>
                  Boot your application inside <code className="font-mono">AppCell</code> or{' '}
                  <code className="font-mono">waft-test</code> in standard <code className="font-mono">cargo test</code>.
                  Drive taps, keyboard events, timers, and async tasks on CI without X11, Wayland, or a GPU.
                </p>
              </div>

              <div className="mt-5">
                <RustCodeBlock code={HEADLESS_TEST_CODE} />
              </div>
            </div>

            {/* Right 6 cols: cargo waft CLI */}
            <div
              className="flex flex-col justify-between rounded-2xl border p-6 sm:p-7 lg:col-span-6"
              style={{
                background: 'var(--site-card)',
                borderColor: 'var(--site-hairline-strong)',
              }}
            >
              <div>
                <div className="flex items-center justify-between gap-2">
                  <h3 className="text-[18px] font-semibold tracking-[-0.015em]" style={{ color: 'var(--site-ink)' }}>
                    One CLI from simulator to signed installer
                  </h3>
                  <Link
                    href="/docs/cli"
                    className="inline-flex items-center gap-1 text-[13px] font-medium transition hover:opacity-75"
                    style={{ color: 'var(--site-ink)' }}
                  >
                    CLI &amp; packaging guide <ArrowUpRight className="size-3.5" />
                  </Link>
                </div>

                <p className="mt-2.5 text-[14px] leading-[1.55]" style={{ color: 'var(--site-muted)' }}>
                  <code className="font-mono">cargo waft</code> scaffolds projects, checks platform SDKs with{' '}
                  <code className="font-mono">doctor</code>, launches on desktop or mobile simulators, and produces
                  signed distributables from a single <code className="font-mono">Cargo.toml</code>.
                </p>
              </div>

              <div className="mt-5">
                <ShellCodeBlock code={CLI_WORKFLOW_CODE} />
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer
        className="mt-auto border-t px-6 py-8 text-[13px]"
        style={{
          borderColor: 'var(--site-hairline)',
          color: 'var(--site-muted)',
        }}
      >
        <div className="mx-auto flex max-w-[1160px] flex-wrap items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <span className="font-semibold" style={{ color: 'var(--site-ink)' }}>
              Waft
            </span>
            <span className="font-mono text-[11.5px]" style={{ color: 'var(--site-subtle)' }}>
              waft.rs
            </span>
          </div>
          <div className="flex flex-wrap items-center gap-6">
            <Link href="/docs/start" className="transition hover:opacity-75">
              Docs
            </Link>
            <a href={galleryUrls.winui} target="_blank" rel="noreferrer" className="transition hover:opacity-75">
              WinUI Gallery
            </a>
            <a href={galleryUrls.cupertino} target="_blank" rel="noreferrer" className="transition hover:opacity-75">
              Cupertino Gallery
            </a>
            <a href={galleryUrls.material} target="_blank" rel="noreferrer" className="transition hover:opacity-75">
              Material Gallery
            </a>
            <Link href="/agents.md" className="transition hover:opacity-75">
              agents.md
            </Link>
            <Link href="/llms.txt" className="transition hover:opacity-75">
              llms.txt
            </Link>
            <a href={githubUrl} target="_blank" rel="noreferrer" className="transition hover:opacity-75">
              GitHub
            </a>
          </div>
        </div>
      </footer>
    </main>
  );
}
