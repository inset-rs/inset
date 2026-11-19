import Link from 'next/link';
import { ArrowRight, ArrowUpRight, CheckCircle2, Terminal, FlaskConical, PackageCheck } from 'lucide-react';
import { githubUrl, galleryUrls, siteOrigin, agentPrompt } from '@/lib/shared';
import { BuildYourOwn } from '@/components/build-your-own';
import { AgentPromptCard } from '@/components/agent-prompt-card';
import { HeroStudio } from '@/components/hero-studio';
import { ArchitectureExplorer } from '@/components/architecture-explorer';
import { DesktopMobileParity } from '@/components/desktop-mobile-parity';

function GitHubMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" className={className}>
      <path d="M12 .5C5.65.5.5 5.65.5 12c0 5.08 3.29 9.39 7.86 10.91.58.11.79-.25.79-.56 0-.28-.01-1.02-.02-2-3.2.7-3.88-1.54-3.88-1.54-.52-1.33-1.28-1.68-1.28-1.68-1.05-.71.08-.7.08-.7 1.16.08 1.77 1.19 1.77 1.19 1.03 1.76 2.7 1.25 3.36.96.1-.75.4-1.25.73-1.54-2.55-.29-5.24-1.28-5.24-5.69 0-1.26.45-2.29 1.19-3.1-.12-.29-.52-1.46.11-3.05 0 0 .97-.31 3.18 1.18a11.1 11.1 0 0 1 5.8 0c2.2-1.49 3.17-1.18 3.17-1.18.63 1.59.23 2.76.11 3.05.74.81 1.19 1.84 1.19 3.1 0 4.42-2.69 5.39-5.25 5.68.41.36.78 1.06.78 2.14 0 1.55-.01 2.8-.01 3.18 0 .31.21.68.8.56A11.51 11.51 0 0 0 23.5 12C23.5 5.65 18.35.5 12 .5Z" />
    </svg>
  );
}

const CLI_STEPS = [
  {
    cmd: 'cargo test',
    badge: '1.4 ms — No display server',
    detail: 'Run widget & state tests headlessly in pure Rust with AppCell and waft-test.',
  },
  {
    cmd: 'cargo waft doctor',
    badge: 'Toolchain check',
    detail: 'Verifies Xcode, Android NDK, Vulkan/Metal SDKs, and target triples.',
  },
  {
    cmd: 'cargo waft run -d ios-sim',
    badge: 'Live device & simulator',
    detail: 'Compiles and boots directly on macOS, Windows, Linux, iOS Simulator, Android, or Web.',
  },
  {
    cmd: 'cargo waft build --release',
    badge: '.dmg — .msi — .deb — .ipa — .apk — .wapk',
    detail: 'Produces signed native bundles and installers from a single Cargo.toml.',
  },
] as const;

export default function HomePage() {
  return (
    <main className="waft-editorial-bg flex flex-1 flex-col">
      {/* 1. Editorial Hero + Live Kit Studio */}
      <section className="relative overflow-hidden px-6 pt-12 pb-16 sm:pt-16 sm:pb-20">
        <div className="mx-auto max-w-[1160px]">
          {/* Editorial top eyebrow */}
          <div className="flex flex-wrap items-center justify-between gap-3 border-b pb-4" style={{ borderColor: 'var(--site-hairline)' }}>
            <div className="inline-flex items-center gap-2 font-mono text-[11.5px] font-semibold uppercase tracking-[0.1em]">
              <span
                className="inline-block size-2 rounded-full"
                style={{ background: 'var(--site-accent)' }}
              />
              <span style={{ color: 'var(--site-ink)' }}>WAFT UI FRAMEWORK</span>
              <span style={{ color: 'var(--site-subtle)' }}>/</span>
              <span style={{ color: 'var(--site-muted)' }}>PURE RUST — VALO + WGPU — #![FORBID(UNSAFE_CODE)]</span>
            </div>
            <div className="hidden items-center gap-4 font-mono text-[11.5px] sm:flex" style={{ color: 'var(--site-muted)' }}>
              <span>Crates: waft — waft-winui — waft-cupertino — waft-material</span>
            </div>
          </div>

          {/* Asymmetric Editorial Title & Lead */}
          <div className="mt-8 grid grid-cols-1 gap-8 lg:grid-cols-12 lg:items-end">
            <div className="lg:col-span-7">
              <h1
                className="text-[clamp(2.35rem,5vw,4.15rem)] leading-[1.03] font-semibold tracking-[-0.04em]"
                style={{ color: 'var(--site-ink)' }}
              >
                Solid, fluent apps in Rust.
                <br />
                <span style={{ color: 'var(--site-muted)' }}>Desktop, mobile, web, and WASI.</span>
              </h1>
            </div>

            <div className="flex flex-col justify-between gap-5 lg:col-span-5">
              <p className="text-[16px] leading-[1.6]" style={{ color: 'var(--site-muted)' }}>
                Flutter’s widget and layout ergonomics in ordinary, customizable Cargo crates. Driven by{' '}
                <code className="font-mono text-[14px]" style={{ color: 'var(--site-ink)' }}>
                  valo
                </code>{' '}
                and{' '}
                <code className="font-mono text-[14px]" style={{ color: 'var(--site-ink)' }}>
                  wgpu
                </code>
                , with mature WinUI, Cupertino, and Material kits that treat mobile touch, IME, and sheets as first-class.
              </p>

              <div className="flex flex-wrap items-center gap-2.5">
                <Link
                  href="/docs/start"
                  className="inline-flex h-10 items-center gap-2 rounded-full px-5 text-[13.5px] font-medium transition hover:opacity-90"
                  style={{
                    background: 'var(--site-ink)',
                    color: 'var(--site-bg)',
                  }}
                >
                  Get started
                  <ArrowRight className="size-3.5" />
                </Link>

                <BuildYourOwn prompt={agentPrompt(undefined, siteOrigin())} startUrl="/docs/start" />

                <a
                  href={githubUrl}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex h-10 items-center gap-2 rounded-full border px-4 text-[13.5px] font-medium transition hover:bg-black/[0.03] dark:hover:bg-white/[0.04]"
                  style={{
                    borderColor: 'var(--site-hairline-strong)',
                    color: 'var(--site-ink)',
                    background: 'var(--site-card)',
                  }}
                >
                  <GitHubMark className="size-4" />
                  GitHub
                </a>
              </div>
            </div>
          </div>

          {/* Interactive Kit Studio right in the hero */}
          <div className="mt-10">
            <HeroStudio />
          </div>
        </div>
      </section>

      {/* 2. Interactive Crate & Embedder Architecture Stack */}
      <ArchitectureExplorer />

      {/* 3. Desktop & Mobile Parity + Pure Rust / valo / wgpu Performance */}
      <DesktopMobileParity />

      {/* 4. AI-Friendly CLI, Headless Testing & Agent Playbook */}
      <section className="px-6 py-20 sm:py-24">
        <div className="mx-auto max-w-[1160px]">
          <div className="flex flex-col justify-between gap-4 md:flex-row md:items-end">
            <div>
              <p
                className="font-mono text-[12px] font-semibold uppercase tracking-[0.12em]"
                style={{ color: 'var(--site-accent)' }}
              >
                03 — Headless Testing &amp; AI-Native Workflow
              </p>
              <h2
                className="mt-2 text-[clamp(1.85rem,3.6vw,2.75rem)] leading-[1.08] font-semibold tracking-[-0.03em]"
                style={{ color: 'var(--site-ink)' }}
              >
                Verify in milliseconds without a display.
                <br className="hidden sm:inline" /> Package everywhere with cargo waft.
              </h2>
            </div>
            <p className="max-w-[440px] text-[15px] leading-[1.6]" style={{ color: 'var(--site-muted)' }}>
              Self-explanatory Rust types, headless test harnesses, and a single packaging CLI make Waft effortless for
              both humans and coding agents to iterate in tight feedback loops.
            </p>
          </div>

          {/* Split: Left Headless Test harness, Right cargo waft workflow */}
          <div className="mt-10 grid grid-cols-1 gap-6 lg:grid-cols-12">
            {/* Left 6 cols: Headless test code */}
            <div
              className="flex flex-col justify-between rounded-2xl border p-6 sm:p-7 lg:col-span-6"
              style={{
                background: 'var(--site-card)',
                borderColor: 'var(--site-hairline-strong)',
              }}
            >
              <div>
                <div className="flex items-center justify-between">
                  <span className="inline-flex items-center gap-2 font-mono text-[12px] font-semibold" style={{ color: 'var(--site-ink)' }}>
                    <FlaskConical className="size-4" style={{ color: 'var(--site-accent)' }} />
                    tests/headless_ui.rs
                  </span>
                  <span className="inline-flex items-center gap-1 rounded-full bg-emerald-500/12 px-2.5 py-0.5 font-mono text-[11px] font-medium text-emerald-700 dark:text-emerald-300">
                    <CheckCircle2 className="size-3" /> PASS — 1.4 ms
                  </span>
                </div>

                <p className="mt-2.5 text-[13.5px] leading-[1.55]" style={{ color: 'var(--site-muted)' }}>
                  Boot your application inside <code className="font-mono">AppCell</code> or{' '}
                  <code className="font-mono">waft-test</code> in standard <code className="font-mono">cargo test</code>.
                  Drive taps, keyboard events, and async tasks on CI without X11, Wayland, or a GPU.
                </p>

                <pre
                  className="mt-4 overflow-x-auto rounded-xl border p-4 font-mono text-[12px] leading-[19px]"
                  style={{
                    background: '#fcfbf9',
                    borderColor: 'var(--site-hairline)',
                    color: '#1c1b18',
                  }}
                >
                  <code>{`#[test]
fn audio_pipeline_toggles_cleanly() {
    let mut app = AppCell::new();
    let studio = StudioSession::new(&mut app);

    studio.toggle_low_latency(&mut app);
    assert!(studio.is_exclusive_buffer(&app));

    studio.export_preset(&mut app, "master.preset");
    assert_eq!(studio.exported_count(&app), 1);
}`}</code>
                </pre>
              </div>

              <div className="mt-4 flex items-center justify-between text-[12.5px]">
                <span className="font-mono text-[11.5px]" style={{ color: 'var(--site-subtle)' }}>
                  No window server required
                </span>
                <Link
                  href="/docs/testing"
                  className="inline-flex items-center gap-1 font-medium"
                  style={{ color: 'var(--site-ink)' }}
                >
                  Read the testing guide <ArrowUpRight className="size-3.5" />
                </Link>
              </div>
            </div>

            {/* Right 6 cols: cargo waft CLI pipeline */}
            <div
              className="flex flex-col justify-between rounded-2xl border p-6 sm:p-7 lg:col-span-6"
              style={{
                background: 'var(--site-card)',
                borderColor: 'var(--site-hairline-strong)',
              }}
            >
              <div>
                <div className="flex items-center justify-between">
                  <span className="inline-flex items-center gap-2 font-mono text-[12px] font-semibold" style={{ color: 'var(--site-ink)' }}>
                    <Terminal className="size-4" style={{ color: 'var(--site-cobalt)' }} />
                    cargo waft CLI
                  </span>
                  <span className="inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 font-mono text-[11px]" style={{ borderColor: 'var(--site-hairline)', color: 'var(--site-muted)' }}>
                    <PackageCheck className="size-3" /> 7 target platforms
                  </span>
                </div>

                <div className="mt-4 space-y-2.5">
                  {CLI_STEPS.map((item) => (
                    <div
                      key={item.cmd}
                      className="rounded-xl border px-3.5 py-2.5"
                      style={{
                        background: 'var(--site-surface)',
                        borderColor: 'var(--site-hairline)',
                      }}
                    >
                      <div className="flex flex-wrap items-center justify-between gap-2">
                        <code className="font-mono text-[12.5px] font-semibold" style={{ color: 'var(--site-ink)' }}>
                          $ {item.cmd}
                        </code>
                        <span
                          className="rounded px-2 py-0.5 font-mono text-[10.5px] font-medium"
                          style={{ background: 'var(--site-card)', color: 'var(--site-accent)' }}
                        >
                          {item.badge}
                        </span>
                      </div>
                      <p className="mt-1 text-[12px] leading-[1.45]" style={{ color: 'var(--site-muted)' }}>
                        {item.detail}
                      </p>
                    </div>
                  ))}
                </div>
              </div>

              <div className="mt-4 flex items-center justify-between text-[12.5px]">
                <span className="font-mono text-[11.5px]" style={{ color: 'var(--site-subtle)' }}>
                  cargo install waft-cli
                </span>
                <Link
                  href="/docs/cli"
                  className="inline-flex items-center gap-1 font-medium"
                  style={{ color: 'var(--site-ink)' }}
                >
                  CLI &amp; packaging reference <ArrowUpRight className="size-3.5" />
                </Link>
              </div>
            </div>
          </div>

          {/* Agent Prompt Builder Card */}
          <div className="mt-8">
            <AgentPromptCard origin={siteOrigin()} />
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
              waft.rs — Pure Rust Cross-Platform UI
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
