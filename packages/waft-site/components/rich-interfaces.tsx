'use client';

import { useState } from 'react';
import Link from 'next/link';
import {
  ArrowUpRight,
  AppWindow,
  Film,
  FolderSync,
  Layers,
} from 'lucide-react';
import { RustCodeBlock } from './code-highlight';

type InterfacePillar = {
  id: 'media' | 'compositing' | 'desktop' | 'data';
  title: string;
  subtitle: string;
  icon: typeof Film;
  summary: string;
  capabilities: {
    name: string;
    detail: string;
  }[];
  hostBackends: string[];
  code: string;
};

const INTERFACE_PILLARS: InterfacePillar[] = [
  {
    id: 'media',
    title: 'Hardware Media, Camera & Timed Cues',
    subtitle: 'PlayerView, CameraPreview, Cues',
    icon: Film,
    summary:
      'Hardware-decoded video playback, live multi-format camera sessions, timed subtitle cues, and GPU frame sampling are built directly into waft-embedder and waft-widgets — no method-channel plugin glue required.',
    capabilities: [
      {
        name: 'Player & PlayerView',
        detail:
          'Hardware-decoded playback with audio/subtitle track selection, seek tolerance, and automatic composition layer placement.',
      },
      {
        name: 'CameraSession & CameraPreview',
        detail:
          'Live camera discovery, format negotiation, horizon-level preview layers, photo capture, and movie recording.',
      },
      {
        name: 'SubtitleView & Timed Cues',
        detail:
          'Styled media cues merged with the user’s system caption preferences and rendered crisp over the video rect.',
      },
      {
        name: 'VideoFrameReader & VideoFrameView',
        detail:
          'Sample decoded video or camera frames directly as GPU textures for custom shaders, scopes, or editors.',
      },
    ],
    hostBackends: [
      'AVPlayer / AVCaptureSession',
      'Media3 / Camera2',
      'Media Foundation',
      'GStreamer / PipeWire',
      'WebCodecs / HTMLMediaElement',
      'wasdk:platform/media',
    ],
    code: `use waft_widgets::{
    CameraOutputs, CameraPreview, CameraSession, MovieOptions,
    Player, PlayerView, VideoFrameView, open_camera,
};

// Hardware video playback + styled cues on a dedicated composition layer
let player = Player::new(app, "https://cdn.waft.rs/demo.mp4")?;
player.play(app);
let video = PlayerView::new(player.clone());

// Or start a live camera session with zero-copy preview & movie recording
let layer = app.platform().compositor()?.make_layer(false);
let outputs = CameraOutputs::new()
    .preview(layer)
    .movies(MovieOptions::default());
let session = CameraSession::start(app, &*camera, &format, outputs).await?;
let preview = CameraPreview::new(session.clone());`,
  },
  {
    id: 'compositing',
    title: 'Zero-Copy Compositing & GPU Buffers',
    subtitle: 'Compositor, HardwareBuffer',
    icon: Layers,
    summary:
      'Punch through the UI scene with hardware composition layers or import native GPU memory handles directly into the valo rendering pipeline without a single CPU byte copy.',
    capabilities: [
      {
        name: 'Compositor & CompositionLayerView',
        detail:
          'Place video, camera, or external rendering surfaces directly into the OS window compositor tree.',
      },
      {
        name: 'HardwareBuffer & Allocator',
        detail:
          'Import or allocate IOSurface, AHardwareBuffer, dmabuf, and D3D shared textures with composer_overlay flags.',
      },
      {
        name: 'Platform::import_texture',
        detail:
          'Bind external hardware buffers as sampleable valo GPU textures inside any custom RenderBox or canvas pass.',
      },
      {
        name: 'FragmentProgram & ImageFilter',
        detail:
          'Run custom WGSL/SPIR-V fragment shaders, backdrop blurs, and color matrices across any widget subtree.',
      },
    ],
    hostBackends: [
      'IOSurface / CAMetalLayer',
      'AHardwareBuffer / SurfaceControl',
      'dmabuf / wl_surface',
      'DXGI / DirectComposition',
      'OffscreenCanvas / WebGPU',
      'wasdk:platform/compositor',
    ],
    code: `use waft_embedder::{HardwareBufferFormat, HardwareBufferUsage};
use waft_widgets::{CompositionLayerView, Stack};

let platform = app.platform();
let compositor = platform.compositor()?;
let allocator = platform.hardware_buffer_allocator()?;

// Allocate a direct hardware overlay buffer (IOSurface / AHardwareBuffer)
let usage = HardwareBufferUsage::new()
    .gpu_sampled_image()
    .composer_overlay();
let format = HardwareBufferFormat::R8G8B8A8Unorm;
let buffer = allocator.allocate(3840, 2160, format, usage)?;

// Present directly on a hardware composition plane under your Rust UI
let layer = compositor.make_layer(true);
layer.set_hardware_buffer(Some(buffer), None);
let view = Stack::new()
    .children([CompositionLayerView::new(layer).into_widget(), hud]);`,
  },
  {
    id: 'desktop',
    title: 'Multi-Window & Native System Chrome',
    subtitle: 'WindowConfig, PlatformMenuBar',
    icon: AppWindow,
    summary:
      'First-class multi-window orchestration, system menu bars, status-bar tray icons, native file sheets, and interactive notifications are standard framework primitives.',
    capabilities: [
      {
        name: 'MultiViewApp & WindowScope',
        detail:
          'Request secondary windows with transparent/blurred backgrounds, custom traffic-light insets, and window levels.',
      },
      {
        name: 'PlatformMenuBar & DockMenu',
        detail:
          'Declarative system menu bars, macOS dock menus, and native context popup menus bound to widget Actions.',
      },
      {
        name: 'PlatformTrayIcon',
        detail:
          'Lifecycle-managed system tray and menu-bar extra icons that mount and unmount cleanly with your widget tree.',
      },
      {
        name: 'FileDialogs & Notifications',
        detail:
          'Parent-sheet file/folder pickers that cancel automatically on drop, plus notifications with inline reply actions.',
      },
    ],
    hostBackends: [
      'AppKit / NSStatusItem / UNUserNotification',
      'Win32 / Shell_NotifyIcon / IFileDialog',
      'Wayland / XDG Desktop Portal / StatusNotifier',
      'UIKit UIScene / Android Activity Tasks',
      'wasdk:platform/window & menus',
    ],
    code: `use waft_embedder::{OpenOptions, WindowBackground, WindowConfig};
use waft_widgets::{PlatformMenuBar, PlatformTrayIcon, WindowScope};

// Open a secondary utility window with native vibrancy
app.platform().request_view(WindowConfig {
    title: "Inspector".into(),
    size: [420.0, 680.0],
    background: WindowBackground::Blurred,
    ..WindowConfig::default()
});

// Parent-anchored native file dialog (dropping the future cancels the sheet)
let dialogs = app.platform().file_dialogs()?;
let options = OpenOptions::new().writable(true);
let picked = dialogs.open_file(Some(view_id), options).await?;

// Mount a system tray icon and menu straight from the widget tree
let shell = PlatformTrayIcon::new(tray_icon)
    .menus(menu_items)
    .child(content);`,
  },
  {
    id: 'data',
    title: 'Rich Clipboard, Drag & Drop, HTTP & Files',
    subtitle: 'Clipboard, DropListener, HttpClient',
    icon: FolderSync,
    summary:
      'Multi-MIME clipboard items, lazy file promises, external OS drag-and-drop streams, random-access files, and streaming HTTP live in the same portable embedder boundary.',
    capabilities: [
      {
        name: 'Multi-Format Clipboard & Promises',
        detail:
          'Read and write multiple MIME representations at once, or register deferred ContentFuture and FileSink promises.',
      },
      {
        name: 'DropListener & DataTransferItem',
        detail:
          'Inspect dragged MIME types on hover and stream dropped files or custom payloads from outside the application.',
      },
      {
        name: 'Streaming HttpClient',
        detail:
          'Built-in async HTTP requests and streaming bodies with typed DNS/TLS errors, timeouts, and trailers.',
      },
      {
        name: 'Random-Access Files & Opener',
        detail:
          'Async read_at/write_at file handles, recursive directory operations, and system URL/file reveal in Finder or Explorer.',
      },
    ],
    hostBackends: [
      'NSPasteboard / NSFilePromiseProvider',
      'OLE IDataObject / WinHTTP',
      'Wayland data-device / X11 selections',
      'URLSession / Cronet / Fetch',
      'wasdk:platform/clipboard & http',
    ],
    code: `use waft_embedder::{ClipboardItem, CopyOptions};
use waft_widgets::DropListener;

// Receive multi-format items dragged from Finder, Explorer, or another app
let drop = DropListener::new(app).on_dropped(|app, items| {
    for item in items {
        if item.types().iter().any(|t| t == "text/uri-list") {
            let read = item.read("text/uri-list");
            app.spawn(async move |_cx| { /* stream dropped file URIs */ });
        }
    }
});

// Promise a large export file on the clipboard — generated only when pasted
let item = ClipboardItem::new().promised_file("recording.wav", || {
    Box::new(async move |sink| sink.write(&rendered_pcm).await)
});
let clipboard = app.platform().clipboard()?;
clipboard.write(vec![item], CopyOptions::new()).await?;`,
  },
];

export function RichInterfaces() {
  const [activeId, setActiveId] = useState<InterfacePillar['id']>('media');
  const active =
    INTERFACE_PILLARS.find((pillar) => pillar.id === activeId) ?? INTERFACE_PILLARS[0];

  return (
    <section
      className="border-b px-4 py-16 sm:px-6 sm:py-20"
      style={{ borderColor: 'var(--site-hairline)' }}
    >
      <div className="mx-auto max-w-[1160px]">
        {/* Section header */}
        <div className="grid grid-cols-1 gap-6 lg:grid-cols-12 lg:items-end">
          <div className="lg:col-span-7">
            <h2
              className="text-[32px] leading-[1.08] font-semibold tracking-[-0.03em] sm:text-[42px]"
              style={{ color: 'var(--site-ink)' }}
            >
              Rich media, zero-copy compositing, and desktop APIs built in.
            </h2>
          </div>
          <p
            className="text-[15.5px] leading-[1.6] lg:col-span-5"
            style={{ color: 'var(--site-muted)' }}
          >
            Most UI frameworks stop at drawing pixels and push camera, video, hardware surfaces,
            windowing, menus, and file dialogs onto fragmented native plugins. Waft defines all of
            them in <code className="font-mono">waft-embedder</code> — portable across native hosts,{' '}
            <code className="font-mono">waft-test</code>, and WASI (<code className="font-mono">wasdk:platform/*</code>).
          </p>
        </div>

        {/* Category selector tabs */}
        <div className="mt-9 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
          {INTERFACE_PILLARS.map((pillar) => {
            const Icon = pillar.icon;
            const isSelected = pillar.id === active.id;
            return (
              <button
                key={pillar.id}
                type="button"
                onClick={() => setActiveId(pillar.id)}
                className="group flex flex-col justify-between rounded-xl border p-4 text-left transition-all"
                style={{
                  background: isSelected ? 'var(--site-card)' : 'var(--site-surface)',
                  borderColor: isSelected
                    ? 'var(--site-ink)'
                    : 'var(--site-hairline-strong)',
                }}
              >
                <div className="flex items-center justify-between gap-2">
                  <span
                    className="inline-flex size-8 items-center justify-center rounded-lg border"
                    style={{
                      background: isSelected ? 'var(--site-accent-soft)' : 'var(--site-card)',
                      borderColor: isSelected
                        ? 'var(--site-accent)'
                        : 'var(--site-hairline-strong)',
                      color: isSelected ? 'var(--site-accent)' : 'var(--site-ink)',
                    }}
                  >
                    <Icon className="size-4" />
                  </span>
                </div>
                <div className="mt-3.5">
                  <div
                    className="text-[15px] font-semibold tracking-[-0.01em]"
                    style={{ color: 'var(--site-ink)' }}
                  >
                    {pillar.title}
                  </div>
                  <div
                    className="mt-1 truncate font-mono text-[11.5px]"
                    style={{ color: 'var(--site-muted)' }}
                  >
                    {pillar.subtitle}
                  </div>
                </div>
              </button>
            );
          })}
        </div>

        {/* Active pillar detail card */}
        <div
          className="mt-5 rounded-2xl border p-6 sm:p-8"
          style={{
            background: 'var(--site-card)',
            borderColor: 'var(--site-hairline-strong)',
          }}
        >
          <div className="grid grid-cols-1 gap-8 lg:grid-cols-12 lg:items-start">
            {/* Left 5 cols: Capability breakdown */}
            <div className="lg:col-span-5">
              <div className="flex items-center justify-between gap-3">
                <h3
                  className="text-[20px] font-semibold tracking-[-0.02em]"
                  style={{ color: 'var(--site-ink)' }}
                >
                  {active.title}
                </h3>
                <Link
                  href="/docs/embedder"
                  className="inline-flex shrink-0 items-center gap-1 text-[13px] font-medium transition hover:opacity-75"
                  style={{ color: 'var(--site-ink)' }}
                >
                  Embedder spec <ArrowUpRight className="size-3.5" />
                </Link>
              </div>

              <p
                className="mt-2.5 text-[14px] leading-[1.58]"
                style={{ color: 'var(--site-muted)' }}
              >
                {active.summary}
              </p>

              <div
                className="mt-5 divide-y border-y"
                style={{
                  borderColor: 'var(--site-hairline)',
                }}
              >
                {active.capabilities.map((cap) => (
                  <div
                    key={cap.name}
                    className="py-3"
                    style={{ borderColor: 'var(--site-hairline)' }}
                  >
                    <div
                      className="font-mono text-[12.5px] font-semibold"
                      style={{ color: 'var(--site-ink)' }}
                    >
                      {cap.name}
                    </div>
                    <p
                      className="mt-0.5 text-[13px] leading-[1.5]"
                      style={{ color: 'var(--site-muted)' }}
                    >
                      {cap.detail}
                    </p>
                  </div>
                ))}
              </div>

              <div className="mt-4">
                <div
                  className="text-[11.5px] font-medium"
                  style={{ color: 'var(--site-subtle)' }}
                >
                  Native &amp; WASI host implementations:
                </div>
                <div className="mt-2 flex flex-wrap gap-1.5">
                  {active.hostBackends.map((backend) => (
                    <span
                      key={backend}
                      className="rounded-md border px-2 py-0.5 font-mono text-[11px]"
                      style={{
                        background: 'var(--site-surface)',
                        borderColor: 'var(--site-hairline-strong)',
                        color: 'var(--site-muted)',
                      }}
                    >
                      {backend}
                    </span>
                  ))}
                </div>
              </div>
            </div>

            {/* Right 7 cols: Real Rust code */}
            <div className="lg:col-span-7">
              <RustCodeBlock code={active.code} />

              <div
                className="mt-3.5 flex flex-wrap items-center justify-between gap-2 rounded-xl border px-4 py-2.5 text-[12.5px]"
                style={{
                  background: 'var(--site-surface)',
                  borderColor: 'var(--site-hairline)',
                  color: 'var(--site-muted)',
                }}
              >
                <span>
                  Every capability includes a deterministic mock in{' '}
                  <code className="font-mono" style={{ color: 'var(--site-ink)' }}>
                    waft_embedder::test_support
                  </code>{' '}
                  for headless unit tests.
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
          </div>
        </div>
      </div>
    </section>
  );
}
