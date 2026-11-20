'use client';

import { useState } from 'react';
import { Check, Copy, ExternalLink, Layers, Monitor, Smartphone } from 'lucide-react';
import { galleryUrls } from '@/lib/shared';
import { RustCodeBlock } from '@/components/code-highlight';

type KitId = 'winui' | 'cupertino' | 'material';

interface KitSpec {
  id: KitId;
  label: string;
  crateName: string;
  galleryUrl: string;
  galleryLabel: string;
  code: string;
}

const KITS: KitSpec[] = [
  {
    id: 'winui',
    label: 'Fluent / WinUI',
    crateName: 'waft-winui',
    galleryUrl: galleryUrls.winui,
    galleryLabel: 'WinUI Gallery',
    code: `use waft_foundation::{App, Handle};
use waft_widgets::*;
use waft_winui::*;

#[waft::main]
fn main(app: &mut App) {
    winui_gallery::run(app);
}

impl State for GalleryRootState {
    type Widget = GalleryRoot;
    waft_widgets::state_accessors!();

    fn build(
        self: Handle<Self>,
        app: &mut App,
        _cx: BuildContext,
    ) -> WidgetRef {
        let feature = self.widget(app).initial_feature;
        let mode = if app.get(self).dark {
            FluentThemeMode::Dark
        } else {
            FluentThemeMode::Light
        };
        FluentApp::new()
            .theme_mode(mode)
            .title("WinUI Gallery")
            .home(Gallery::new(feature))
            .into_widget()
    }
}`,
  },
  {
    id: 'cupertino',
    label: 'Cupertino',
    crateName: 'waft-cupertino',
    galleryUrl: galleryUrls.cupertino,
    galleryLabel: 'Cupertino Gallery',
    code: `use std::rc::Rc;
use waft_cupertino::{CupertinoApp, CupertinoPageRoute};
use waft_foundation::App;
use waft_widgets::*;

#[waft::main]
fn main(app: &mut App) {
    cupertino_gallery::run_gallery(app, None);
}

pub fn gallery() -> CupertinoApp {
    CupertinoApp::new()
        .title("Cupertino Gallery")
        .on_generate_route(|app, settings| {
            let root = settings.name.as_deref() == Some("/");
            root.then(|| {
                let page: WidgetBuilder = Rc::new(|_, _| {
                    pages::home::Home.into_widget()
                });
                CupertinoPageRoute::new(app, page)
                    .title(app, "Gallery".to_owned())
                    .into_erased()
            })
        })
}`,
  },
  {
    id: 'material',
    label: 'Material 3',
    crateName: 'waft-material',
    galleryUrl: galleryUrls.material,
    galleryLabel: 'Material Gallery',
    code: `use waft_foundation::{App, Handle};
use waft_material::{MaterialApp, ThemeData};
use waft_widgets::*;

#[waft::main]
fn main(app: &mut App) {
    material_gallery::run_gallery(app, None, None);
}

impl State for GalleryAppState {
    type Widget = GalleryApp;
    waft_widgets::state_accessors!();

    fn build(
        self: Handle<Self>,
        app: &mut App,
        _cx: BuildContext,
    ) -> WidgetRef {
        let theme = ThemeData::from_seed(self.seed(app));
        MaterialApp::new()
            .title("Material 3 Gallery")
            .theme(theme)
            .home(GalleryHome::new(self))
            .into_widget()
    }
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
    void navigator.clipboard?.writeText('cargo install waft-cli && cargo waft new notes');
    setCopiedCmd(true);
    setTimeout(() => setCopiedCmd(false), 1800);
  };

  return (
    <div className="mt-10">
      <div
        className="overflow-hidden rounded-2xl border shadow-[0_24px_60px_-24px_rgba(20,20,19,0.12)]"
        style={{
          background: 'var(--site-card)',
          borderColor: 'var(--site-hairline-strong)',
        }}
      >
        {/* Top Studio Bar: Kit Switcher + Gallery Link + CLI Quickstart */}
        <div
          className="flex flex-wrap items-center justify-between gap-3 border-b px-4 py-3 sm:px-6"
          style={{
            background: 'var(--site-surface)',
            borderColor: 'var(--site-hairline)',
          }}
        >
          {/* Left: Design System Kit Tabs */}
          <div className="flex flex-wrap items-center gap-1.5" >
            {KITS.map((k) => {
              const selected = k.id === activeKit;
              const Icon = k.id === 'winui' ? Monitor : k.id === 'cupertino' ? Smartphone : Layers;
              return (
                <button
                  key={k.id}
                  type="button"
                  onClick={() => setActiveKit(k.id)}
                  className="inline-flex items-center gap-2 rounded-lg border px-3 py-1.5 text-[12.5px] font-medium transition-all"
                  style={{
                    background: selected ? 'var(--site-card)' : 'transparent',
                    borderColor: selected ? 'var(--site-hairline-strong)' : 'transparent',
                    color: selected ? 'var(--site-ink)' : 'var(--site-muted)',
                    boxShadow: selected ? '0 1px 2px rgba(20, 20, 19, 0.06)' : 'none',
                  }}
                >
                  <Icon
                    className="size-3.5"
                    style={{ color: selected ? 'var(--site-accent)' : 'var(--site-subtle)' }}
                  />
                  <span>{k.label}</span>
                  <span
                    className="hidden rounded px-1.5 py-0.5 font-mono text-[10.5px] sm:inline-block"
                    style={{
                      background: selected ? 'var(--site-accent-soft)' : 'rgba(20,20,19,0.05)',
                      color: selected ? 'var(--site-accent)' : 'var(--site-subtle)',
                    }}
                  >
                    {k.crateName}
                  </span>
                </button>
              );
            })}
          </div>

          {/* Right: Live WebGPU Gallery & CLI command copy */}
          <div className="flex flex-wrap items-center gap-2">
            <a
              href={spec.galleryUrl}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-1.5 rounded-lg border px-2.5 py-1 text-[11.5px] font-medium transition hover:opacity-80"
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

        {/* Main Split: Rust Code Left (5 cols) | Live WebGPU Gallery Right (7 cols) */}
        <div className="grid grid-cols-1 lg:grid-cols-12">
          {/* Left: Rust Code View */}
          <div
            className="flex flex-col justify-between border-b p-5 sm:p-6 lg:col-span-5 lg:border-r lg:border-b-0"
            style={{
              background: 'var(--site-code-bg)',
              borderColor: 'var(--site-hairline)',
            }}
          >
            <RustCodeBlock code={spec.code} className="!border-0 !bg-transparent !p-0" />
          </div>

          {/* Right: Real Live WebGPU Gallery Embed */}
          <div
            className="relative flex min-h-[580px] flex-col overflow-hidden lg:col-span-7"
            style={{
              background: activeKit === 'cupertino' ? 'var(--site-surface)' : '#ffffff',
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
                  className="relative h-[540px] w-full max-w-[390px] overflow-hidden rounded-[28px] border bg-white shadow-[0_18px_45px_-18px_rgba(20,20,19,0.2)]"
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
