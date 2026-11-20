import Link from 'next/link';
import {
  AppWindow,
  ArrowRight,
  ArrowUpRight,
  Camera,
  ClipboardCopy,
  ExternalLink,
  Film,
  Layers,
} from 'lucide-react';

interface CapabilityCard {
  id: 'media' | 'compositing' | 'windowing' | 'data';
  title: string;
  description: string;
  types: string[];
  docsHref: string;
  docsRsHref: string;
}

const CAPABILITIES: CapabilityCard[] = [
  {
    id: 'media',
    title: 'Hardware Media, Camera & Subtitles',
    description:
      'Hardware-decoded video playback, live camera capture and recording, styled subtitle cues, and direct GPU frame sampling without method-channel plugins.',
    types: ['PlayerView', 'CameraPreview', 'SubtitleView', 'VideoFrameReader'],
    docsHref: '/docs/embedder',
    docsRsHref: 'https://docs.rs/waft-embedder/latest/waft_embedder/',
  },
  {
    id: 'compositing',
    title: 'Zero-Copy Compositing & GPU Buffers',
    description:
      'Present HardwareBuffer surfaces directly onto OS compositor overlay planes under or over your Rust UI, or bind external GPU textures straight into valo shaders.',
    types: ['Compositor', 'CompositionLayer', 'HardwareBuffer'],
    docsHref: '/docs/embedder',
    docsRsHref: 'https://docs.rs/waft-embedder/latest/waft_embedder/',
  },
  {
    id: 'windowing',
    title: 'Multi-Window & Native Desktop Shell',
    description:
      'Spawn secondary windows with native vibrancy, attach system menu bars and tray icons, open parent-anchored file sheets, and send native notifications.',
    types: ['WindowConfig', 'PlatformMenuBar', 'PlatformTrayIcon', 'FileDialogs'],
    docsHref: '/docs/embedder',
    docsRsHref: 'https://docs.rs/waft-embedder/latest/waft_embedder/',
  },
  {
    id: 'data',
    title: 'Rich Clipboard, Drag & Drop, HTTP & Files',
    description:
      'Multi-MIME clipboard items with deferred file promises, external OS drag-and-drop streams, random-access file I/O, and streaming HTTP built into the host traits.',
    types: ['Clipboard', 'DropListener', 'HttpClient', 'Files'],
    docsHref: '/docs/embedder',
    docsRsHref: 'https://docs.rs/waft-embedder/latest/waft_embedder/',
  },
];

function SchematicHeader({ id }: { id: CapabilityCard['id'] }) {
  if (id === 'media') {
    return (
      <div
        className="flex flex-wrap items-center justify-between gap-2 rounded-xl border px-3.5 py-3"
        style={{
          background: 'var(--site-surface)',
          borderColor: 'var(--site-hairline)',
        }}
      >
        <div className="flex flex-wrap items-center gap-2">
          <span
            className="inline-flex size-7 shrink-0 items-center justify-center rounded-lg"
            style={{ background: 'var(--site-accent-soft)', color: 'var(--site-accent)' }}
          >
            <Film className="size-3.5" />
          </span>
          <span
            className="rounded-md border px-2 py-0.5 font-mono text-[11px]"
            style={{
              background: 'var(--site-card)',
              borderColor: 'var(--site-hairline-strong)',
              color: 'var(--site-ink)',
            }}
          >
            AVPlayer / Media3 / WMF / WebCodecs
          </span>
          <ArrowRight className="size-3.5 shrink-0" style={{ color: 'var(--site-subtle)' }} />
          <span
            className="inline-flex items-center gap-1 rounded-md border px-2 py-0.5 font-mono text-[11px] font-semibold"
            style={{
              background: 'var(--site-accent-soft)',
              borderColor: 'rgba(214, 74, 19, 0.25)',
              color: 'var(--site-accent)',
            }}
          >
            <Camera className="size-3" />
            4K60 + Cues
          </span>
        </div>
      </div>
    );
  }

  if (id === 'compositing') {
    return (
      <div
        className="flex flex-wrap items-center justify-between gap-2 rounded-xl border px-3.5 py-3"
        style={{
          background: 'var(--site-surface)',
          borderColor: 'var(--site-hairline)',
        }}
      >
        <div className="flex flex-wrap items-center gap-1.5">
          <span
            className="inline-flex size-7 shrink-0 items-center justify-center rounded-lg"
            style={{ background: 'var(--site-accent-soft)', color: 'var(--site-accent)' }}
          >
            <Layers className="size-3.5" />
          </span>
          {['IOSurface', 'AHardwareBuffer', 'dmabuf'].map((label) => (
            <span
              key={label}
              className="rounded-md border px-2 py-0.5 font-mono text-[11px]"
              style={{
                background: 'var(--site-card)',
                borderColor: 'var(--site-hairline-strong)',
                color: 'var(--site-ink)',
              }}
            >
              {label}
            </span>
          ))}
        </div>
        <span
          className="rounded-md border px-2 py-0.5 font-mono text-[11px] font-semibold"
          style={{
            background: 'var(--site-accent-soft)',
            borderColor: 'rgba(214, 74, 19, 0.25)',
            color: 'var(--site-accent)',
          }}
        >
          0 CPU copies
        </span>
      </div>
    );
  }

  if (id === 'windowing') {
    return (
      <div
        className="flex flex-wrap items-center justify-between gap-2 rounded-xl border px-3.5 py-3"
        style={{
          background: 'var(--site-surface)',
          borderColor: 'var(--site-hairline)',
        }}
      >
        <div className="flex flex-wrap items-center gap-2">
          <span
            className="inline-flex size-7 shrink-0 items-center justify-center rounded-lg"
            style={{ background: 'var(--site-accent-soft)', color: 'var(--site-accent)' }}
          >
            <AppWindow className="size-3.5" />
          </span>
          {['MenuBar', 'TrayIcon', 'FileDialogs'].map((item) => (
            <span
              key={item}
              className="rounded-md border px-2 py-0.5 font-mono text-[11px]"
              style={{
                background: 'var(--site-card)',
                borderColor: 'var(--site-hairline-strong)',
                color: 'var(--site-ink)',
              }}
            >
              {item}
            </span>
          ))}
        </div>
        <span
          className="rounded-md border px-2 py-0.5 font-mono text-[11px] font-semibold"
          style={{
            background: 'var(--site-accent-soft)',
            borderColor: 'rgba(214, 74, 19, 0.25)',
            color: 'var(--site-accent)',
          }}
        >
          Native vibrancy
        </span>
      </div>
    );
  }

  return (
    <div
      className="flex flex-wrap items-center justify-between gap-2 rounded-xl border px-3.5 py-3"
      style={{
        background: 'var(--site-surface)',
        borderColor: 'var(--site-hairline)',
      }}
    >
      <div className="flex flex-wrap items-center gap-2">
        <span
          className="inline-flex size-7 shrink-0 items-center justify-center rounded-lg"
          style={{ background: 'var(--site-accent-soft)', color: 'var(--site-accent)' }}
        >
          <ClipboardCopy className="size-3.5" />
        </span>
        <span
          className="rounded-md border px-2 py-0.5 font-mono text-[11px]"
          style={{
            background: 'var(--site-card)',
            borderColor: 'var(--site-hairline-strong)',
            color: 'var(--site-ink)',
          }}
        >
          Drop / File Promises
        </span>
        <ArrowRight className="size-3.5 shrink-0" style={{ color: 'var(--site-subtle)' }} />
        <span
          className="rounded-md border px-2 py-0.5 font-mono text-[11px] font-semibold"
          style={{
            background: 'var(--site-accent-soft)',
            borderColor: 'rgba(214, 74, 19, 0.25)',
            color: 'var(--site-accent)',
          }}
        >
          Async Streams
        </span>
      </div>
    </div>
  );
}

export function RichInterfaces() {
  return (
    <section
      className="border-b px-4 py-16 sm:px-6 sm:py-20"
      style={{
        background: 'var(--site-bg)',
        borderColor: 'var(--site-hairline)',
      }}
    >
      <div className="mx-auto max-w-[1160px]">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <h2
            className="text-[32px] leading-[1.08] font-semibold tracking-[-0.03em] sm:text-[42px]"
            style={{ color: 'var(--site-ink)' }}
          >
            Media, zero-copy compositing, and desktop APIs built in.
            <br />
            <span style={{ color: 'var(--site-muted)' }}>No third-party plugin maze.</span>
          </h2>

          <Link
            href="/docs/embedder"
            className="inline-flex items-center gap-1.5 rounded-full border px-4 py-2 text-[13px] font-medium transition hover:opacity-80"
            style={{
              background: 'var(--site-card)',
              borderColor: 'var(--site-hairline-strong)',
              color: 'var(--site-ink)',
            }}
          >
            Embedder spec
            <ArrowUpRight className="size-3.5" />
          </Link>
        </div>

        {/* 2x2 Visual Schematic Bento — zero code blocks */}
        <div className="mt-10 grid grid-cols-1 gap-5 lg:grid-cols-2">
          {CAPABILITIES.map((cap) => (
            <div
              key={cap.id}
              className="flex flex-col justify-between rounded-2xl border p-6"
              style={{
                background: 'var(--site-card)',
                borderColor: 'var(--site-hairline-strong)',
              }}
            >
              <div>
                <SchematicHeader id={cap.id} />

                <h3
                  className="mt-5 text-[18px] font-semibold tracking-[-0.015em]"
                  style={{ color: 'var(--site-ink)' }}
                >
                  {cap.title}
                </h3>
                <p className="mt-2 text-[14px] leading-[1.6]" style={{ color: 'var(--site-muted)' }}>
                  {cap.description}
                </p>
              </div>

              <div
                className="mt-6 flex flex-wrap items-center justify-between gap-3 border-t pt-4"
                style={{ borderColor: 'var(--site-hairline)' }}
              >
                <div className="flex flex-wrap gap-1.5">
                  {cap.types.map((typeName) => (
                    <span
                      key={typeName}
                      className="rounded-md px-2 py-0.5 font-mono text-[11px]"
                      style={{
                        background: 'var(--site-surface)',
                        color: 'var(--site-muted)',
                      }}
                    >
                      {typeName}
                    </span>
                  ))}
                </div>

                <a
                  href={cap.docsRsHref}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex shrink-0 items-center gap-1 font-mono text-[12px] font-medium transition hover:opacity-75"
                  style={{ color: 'var(--site-accent)' }}
                >
                  docs.rs
                  <ExternalLink className="size-3" />
                </a>
              </div>
            </div>
          ))}
        </div>

        {/* Portability note */}
        <div
          className="mt-6 flex flex-wrap items-center justify-between gap-3 rounded-xl border px-5 py-3.5 text-[13px]"
          style={{
            background: 'var(--site-surface)',
            borderColor: 'var(--site-hairline)',
            color: 'var(--site-muted)',
          }}
        >
          <span>
            Every capability is defined in <code className="font-mono">waft-embedder</code> with deterministic mocks in{' '}
            <code className="font-mono">waft-test</code> and portable bindings for native hosts and WASI.
          </span>
          <Link
            href="/docs/testing"
            className="inline-flex items-center gap-1 font-medium transition hover:opacity-75"
            style={{ color: 'var(--site-ink)' }}
          >
            Headless mocks <ArrowUpRight className="size-3.5" />
          </Link>
        </div>
      </div>
    </section>
  );
}
