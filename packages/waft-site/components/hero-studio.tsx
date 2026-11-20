'use client';

import { useState } from 'react';
import { Check, Copy, ExternalLink, Layers, Monitor, Smartphone } from 'lucide-react';
import { galleryUrls, githubUrl } from '@/lib/shared';
import { RustCodeBlock } from '@/components/code-highlight';

type KitId = 'winui' | 'cupertino' | 'material';

interface KitSpec {
  id: KitId;
  label: string;
  crateName: string;
  docsRsUrl: string;
  examplePath: string;
  galleryUrl: string;
  galleryLabel: string;
  code: string;
}

const KITS: KitSpec[] = [
  {
    id: 'winui',
    label: 'Fluent / WinUI',
    crateName: 'waft-winui',
    docsRsUrl: 'https://docs.rs/waft-winui/latest/waft_winui/',
    examplePath: 'examples/winui_gallery',
    galleryUrl: galleryUrls.winui,
    galleryLabel: 'WinUI Gallery',
    code: `use waft::{App, IntoWidget, run_app};
use waft_winui::{FluentApp, FluentThemeMode};

#[waft::main]
fn main(app: &mut App) {
    let ui = FluentApp::new()
        .title("WinUI Gallery")
        .theme_mode(FluentThemeMode::Light)
        .home(Gallery::new())
        .into_widget();

    run_app(app, ui);
}`,
  },
  {
    id: 'cupertino',
    label: 'Cupertino',
    crateName: 'waft-cupertino',
    docsRsUrl: 'https://docs.rs/waft-cupertino/latest/waft_cupertino/',
    examplePath: 'examples/cupertino_gallery',
    galleryUrl: galleryUrls.cupertino,
    galleryLabel: 'Cupertino Gallery',
    code: `use waft::{App, IntoWidget, run_app};
use waft_cupertino::{
    CupertinoApp, CupertinoPageScaffold,
};

#[waft::main]
fn main(app: &mut App) {
    let ui = CupertinoApp::new()
        .title("Cupertino Gallery")
        .home(CupertinoPageScaffold::new(Home))
        .into_widget();

    run_app(app, ui);
}`,
  },
  {
    id: 'material',
    label: 'Material 3',
    crateName: 'waft-material',
    docsRsUrl: 'https://docs.rs/waft-material/latest/waft_material/',
    examplePath: 'examples/material_gallery',
    galleryUrl: galleryUrls.material,
    galleryLabel: 'Material Gallery',
    code: `use waft::{App, IntoWidget, run_app};
use waft_material::{
    MaterialApp, ThemeData, ThemeMode,
};

#[waft::main]
fn main(app: &mut App) {
    let ui = MaterialApp::new()
        .title("Material Gallery")
        .theme(ThemeData::light())
        .dark_theme(ThemeData::dark())
        .theme_mode(ThemeMode::System)
        .home(Gallery::new())
        .into_widget();

    run_app(app, ui);
}`,
  },
];

export function HeroStudio() {
  const [activeKit, setActiveKit] = useState<KitId>('winui');
  const [copiedCmd, setCopiedCmd] = useState(false);
  const [loadedUrl, setLoadedUrl] = useState<string | null>(null);

  const spec = KITS.find((k) => k.id === activeKit) ?? KITS[0];
  const isIframeReady = loadedUrl === spec.galleryUrl;

  const copyCli = () => {
    void navigator.clipboard?.writeText('cargo waft new notes');
    setCopiedCmd(true);
    window.setTimeout(() => setCopiedCmd(false), 1800);
  };

  return (
    <div className="mx-auto w-full max-w-[1160px]">
      <div
        className="overflow-hidden rounded-2xl border shadow-[0_24px_60px_-28px_rgba(20,20,19,0.16)]"
        style={{
          background: 'var(--site-card)',
          borderColor: 'var(--site-hairline-strong)',
        }}
      >
        {/* Top Studio Toolbar: Kit Switcher + Open Gallery + Quick CLI */}
        <div
          className="flex flex-wrap items-center justify-between gap-3 border-b px-4 py-3"
          style={{
            background: 'var(--site-surface)',
            borderColor: 'var(--site-hairline)',
          }}
        >
          <div className="flex flex-wrap items-center gap-1.5" aria-label="Choose UI Kit">
            {KITS.map((kit) => {
              const isActive = kit.id === activeKit;
              const Icon = kit.id === 'winui' ? Monitor : kit.id === 'cupertino' ? Smartphone : Layers;
              return (
                <button
                  key={kit.id}
                  type="button"
                  aria-pressed={isActive}
                  onClick={() => setActiveKit(kit.id)}
                  className="inline-flex items-center gap-2 rounded-lg px-3 py-1.5 text-[13px] font-medium transition"
                  style={
                    isActive
                      ? {
                          background: 'var(--site-card)',
                          color: 'var(--site-ink)',
                          boxShadow: '0 1px 2px rgba(0,0,0,0.06), 0 0 0 1px rgba(20,20,19,0.09)',
                        }
                      : {
                          color: 'var(--site-muted)',
                        }
                  }
                >
                  <Icon
                    className="size-3.5"
                    style={{ color: isActive ? 'var(--site-accent)' : 'var(--site-subtle)' }}
                  />
                  <span>{kit.label}</span>
                  <span
                    className="hidden rounded px-1.5 py-0.5 font-mono text-[10.5px] sm:inline"
                    style={{
                      background: isActive ? 'var(--site-accent-soft)' : 'rgba(20,20,19,0.05)',
                      color: isActive ? 'var(--site-accent)' : 'var(--site-subtle)',
                    }}
                  >
                    {kit.crateName}
                  </span>
                </button>
              );
            })}
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <a
              href={spec.galleryUrl}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-1.5 rounded-lg border px-2.5 py-1 text-[12px] font-medium transition hover:opacity-85"
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
                background: 'var(--site-surface-strong)',
                borderColor: 'var(--site-hairline-strong)',
                color: 'var(--site-ink)',
              }}
              title="Copy cargo waft quickstart command"
            >
              <span style={{ color: 'var(--site-accent)' }}>$</span>
              <span>cargo waft new notes</span>
              {copiedCmd ? (
                <Check className="size-3.5 text-emerald-700" />
              ) : (
                <Copy className="size-3.5" style={{ color: 'var(--site-subtle)' }} />
              )}
            </button>
          </div>
        </div>

        {/* Main Split: Concise First-Glance Rust (40%) | Live WebGPU Gallery (60%) */}
        <div className="grid grid-cols-1 lg:grid-cols-[minmax(360px,40%)_1fr]">
          {/* Left: Concise First-Glance Rust Code */}
          <div
            className="flex flex-col justify-between border-b p-5 sm:p-6 lg:border-r lg:border-b-0"
            style={{
              background: 'var(--site-code-bg)',
              borderColor: 'var(--site-hairline)',
            }}
          >
            <div>
              <RustCodeBlock code={spec.code} className="!border-0 !bg-transparent !p-0" />
            </div>

            <div
              className="mt-6 flex flex-wrap items-center justify-between gap-2 border-t pt-4 text-[12px]"
              style={{
                borderColor: 'var(--site-hairline)',
                color: 'var(--site-muted)',
              }}
            >
              <a
                href={spec.docsRsUrl}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1 font-mono font-medium transition hover:opacity-75"
                style={{ color: 'var(--site-ink)' }}
              >
                <span>docs.rs/{spec.crateName}</span>
                <ExternalLink className="size-3" />
              </a>
              <a
                href={`${githubUrl}/tree/main/${spec.examplePath}`}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1 font-mono transition hover:opacity-75"
                style={{ color: 'var(--site-accent)' }}
              >
                <span>{spec.examplePath}</span>
                <ExternalLink className="size-3" />
              </a>
            </div>
          </div>

          {/* Right: Real Live WebGPU Gallery Embed */}
          <div
            className="relative flex h-[580px] flex-col overflow-hidden"
            style={{
              background: activeKit === 'cupertino' ? 'var(--site-surface)' : '#faf9f6',
            }}
          >
            {!isIframeReady && (
              <div
                className="pointer-events-none absolute inset-0 z-10 flex flex-col items-center justify-center gap-2.5 transition-opacity duration-200"
                style={{
                  background: activeKit === 'cupertino' ? 'var(--site-surface)' : '#faf9f6',
                }}
              >
                <div
                  className="h-1 w-36 overflow-hidden rounded-full"
                  style={{ background: 'var(--site-hairline-strong)' }}
                >
                  <div
                    className="h-full w-2/3 animate-pulse rounded-full"
                    style={{ background: 'var(--site-accent)' }}
                  />
                </div>
                <span
                  className="font-mono text-[11.5px]"
                  style={{ color: 'var(--site-muted)' }}
                >
                  Loading {spec.galleryLabel} (WebGPU)…
                </span>
              </div>
            )}

            {activeKit === 'cupertino' ? (
              <div className="flex flex-1 items-center justify-center p-4 sm:p-6">
                <div
                  className="relative h-[532px] w-full max-w-[390px] overflow-hidden rounded-[28px] border bg-[#faf9f6] shadow-[0_18px_45px_-18px_rgba(20,20,19,0.2)]"
                  style={{ borderColor: 'var(--site-hairline-strong)' }}
                >
                  <iframe
                    key={spec.galleryUrl}
                    src={spec.galleryUrl}
                    title={spec.galleryLabel}
                    onLoad={() => setLoadedUrl(spec.galleryUrl)}
                    className="absolute -inset-[2px] h-[calc(100%+4px)] w-[calc(100%+4px)] border-0 outline-none"
                    allow="clipboard-read; clipboard-write"
                  />
                </div>
              </div>
            ) : (
              <div className="relative h-[580px] w-full flex-1 overflow-hidden">
                <iframe
                  key={spec.galleryUrl}
                  src={spec.galleryUrl}
                  title={spec.galleryLabel}
                  onLoad={() => setLoadedUrl(spec.galleryUrl)}
                  className="absolute -inset-[2px] h-[calc(100%+4px)] w-[calc(100%+4px)] border-0 outline-none"
                  allow="clipboard-read; clipboard-write"
                />
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
