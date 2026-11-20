import {
  AppWindow,
  Camera,
  FileUp,
  Film,
  FolderSync,
  Layers,
} from 'lucide-react';
import { RustCodeBlock } from './code-highlight';

const MEDIA_CODE = `// Hardware video + subtitles on a dedicated composition layer
let player = Player::new(app, "https://cdn.waft.rs/clip.mp4")?;
player.play(app);
let video = PlayerView::new(player);

// Live camera session with zero-copy preview & movie capture
let outputs = CameraOutputs::new().preview(layer);
let session =
    CameraSession::start(app, &*cam, &fmt, outputs).await?;
let preview = CameraPreview::new(session);`;

const COMPOSITING_CODE = `// Allocate a zero-copy GPU buffer (IOSurface / AHardwareBuffer)
let usage = HardwareBufferUsage::new()
    .gpu_sampled_image()
    .composer_overlay();
let buf = allocator.allocate(3840, 2160, format, usage)?;

// Punch through or layer under your Rust UI chrome
let layer = app.platform().compositor()?.make_layer(true);
layer.set_hardware_buffer(Some(buf), None);`;

const DESKTOP_CODE = `// Secondary window with native macOS/Windows vibrancy
app.platform().request_view(WindowConfig {
    title: "Inspector".into(),
    background: WindowBackground::Blurred,
    ..WindowConfig::default()
});

// System tray icon & native file sheet
let picked = dialogs.open_file(Some(view_id), options).await?;
let shell = PlatformTrayIcon::new(icon).menus(items).child(ui);`;

const DATA_CODE = `// Multi-MIME external drag & drop stream
let drop = DropListener::new(app).on_dropped(|app, items| {
    let uris = items[0].read("text/uri-list");
    app.spawn(async move |_cx| { /* stream dropped files */ });
});

// Lazy clipboard file promise — rendered only when pasted
let item = ClipboardItem::new()
    .promised_file("mix.wav", write_fn);
let clipboard = app.platform().clipboard()?;
clipboard.write(vec![item], CopyOptions::new()).await?;`;

export function RichInterfaces() {
  return (
    <section
      className="border-b px-4 py-16 sm:px-6 sm:py-20"
      style={{ borderColor: 'var(--site-hairline)' }}
    >
      <div className="mx-auto max-w-[1160px]">
        <h2
          className="text-[32px] leading-[1.08] font-semibold tracking-[-0.03em] sm:text-[42px]"
          style={{ color: 'var(--site-ink)' }}
        >
          Media, zero-copy compositing, and desktop APIs built in.
          <br />
          <span style={{ color: 'var(--site-muted)' }}>
            No third-party plugin maze.
          </span>
        </h2>

        {/* 2x2 Visual Capability Grid */}
        <div className="mt-10 grid grid-cols-1 gap-6 lg:grid-cols-2">
          {/* Card 1: Hardware Media, Camera & Timed Cues */}
          <div
            className="flex flex-col justify-between rounded-2xl border p-6"
            style={{
              background: 'var(--site-card)',
              borderColor: 'var(--site-hairline-strong)',
            }}
          >
            <div>
              {/* Visual header: Video + Camera + Subtitle Cue Preview */}
              <div
                className="flex flex-wrap items-center justify-between gap-3 rounded-xl border px-4 py-3"
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
                    <Film className="size-3.5" />
                  </span>
                  <div>
                    <div
                      className="font-mono text-[11.5px] font-semibold"
                      style={{ color: 'var(--site-ink)' }}
                    >
                      PlayerView + CameraPreview
                    </div>
                    <div
                      className="font-mono text-[10.5px]"
                      style={{ color: 'var(--site-muted)' }}
                    >
                      AVPlayer / Media3 / WMF / WebCodecs
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-1.5">
                  <span
                    className="inline-flex items-center gap-1 rounded-md border px-2 py-0.5 font-mono text-[10.5px]"
                    style={{
                      background: 'var(--site-card)',
                      borderColor: 'var(--site-hairline-strong)',
                      color: 'var(--site-ink)',
                    }}
                  >
                    <Camera className="size-3" style={{ color: 'var(--site-accent)' }} />
                    4K60
                  </span>
                  <span
                    className="rounded-md border px-2 py-0.5 font-mono text-[10.5px]"
                    style={{
                      background: 'var(--site-card)',
                      borderColor: 'var(--site-hairline-strong)',
                      color: 'var(--site-ink)',
                    }}
                  >
                    CC SubtitleView
                  </span>
                </div>
              </div>

              <div className="mt-4">
                <h3
                  className="text-[19px] font-semibold tracking-[-0.015em]"
                  style={{ color: 'var(--site-ink)' }}
                >
                  Hardware Media, Camera &amp; Subtitles
                </h3>
                <p
                  className="mt-1 text-[13.5px] leading-[1.55]"
                  style={{ color: 'var(--site-muted)' }}
                >
                  Hardware-decoded video, live camera capture, styled subtitle cues, and direct{' '}
                  <code className="font-mono">VideoFrameReader</code> GPU texture sampling.
                </p>
              </div>
            </div>

            <div className="mt-4">
              <RustCodeBlock code={MEDIA_CODE} />
            </div>
          </div>

          {/* Card 2: Zero-Copy Compositing & Hardware Buffers */}
          <div
            className="flex flex-col justify-between rounded-2xl border p-6"
            style={{
              background: 'var(--site-card)',
              borderColor: 'var(--site-hairline-strong)',
            }}
          >
            <div>
              {/* Visual header: Zero-Copy GPU Plane Diagram */}
              <div
                className="flex flex-wrap items-center justify-between gap-2 rounded-xl border px-4 py-3"
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
                    <Layers className="size-3.5" />
                  </span>
                  <div className="flex flex-wrap items-center gap-1.5 font-mono text-[11px]">
                    <span
                      className="rounded border px-1.5 py-0.5 font-medium"
                      style={{
                        background: 'var(--site-card)',
                        borderColor: 'var(--site-hairline-strong)',
                        color: 'var(--site-ink)',
                      }}
                    >
                      IOSurface
                    </span>
                    <span
                      className="rounded border px-1.5 py-0.5 font-medium"
                      style={{
                        background: 'var(--site-card)',
                        borderColor: 'var(--site-hairline-strong)',
                        color: 'var(--site-ink)',
                      }}
                    >
                      AHardwareBuffer
                    </span>
                    <span
                      className="rounded border px-1.5 py-0.5 font-medium"
                      style={{
                        background: 'var(--site-card)',
                        borderColor: 'var(--site-hairline-strong)',
                        color: 'var(--site-ink)',
                      }}
                    >
                      dmabuf
                    </span>
                  </div>
                </div>
                <span
                  className="rounded-md px-2 py-0.5 font-mono text-[10.5px] font-semibold"
                  style={{
                    background: 'var(--site-accent-soft)',
                    color: 'var(--site-accent)',
                  }}
                >
                  0 CPU copies
                </span>
              </div>

              <div className="mt-4">
                <h3
                  className="text-[19px] font-semibold tracking-[-0.015em]"
                  style={{ color: 'var(--site-ink)' }}
                >
                  Zero-Copy Compositing &amp; GPU Buffers
                </h3>
                <p
                  className="mt-1 text-[13.5px] leading-[1.55]"
                  style={{ color: 'var(--site-muted)' }}
                >
                  Present <code className="font-mono">HardwareBuffer</code> surfaces directly on OS compositor overlay
                  planes or bind external textures into <code className="font-mono">valo</code> shaders.
                </p>
              </div>
            </div>

            <div className="mt-4">
              <RustCodeBlock code={COMPOSITING_CODE} />
            </div>
          </div>

          {/* Card 3: Multi-Window & Native System Chrome */}
          <div
            className="flex flex-col justify-between rounded-2xl border p-6"
            style={{
              background: 'var(--site-card)',
              borderColor: 'var(--site-hairline-strong)',
            }}
          >
            <div>
              {/* Visual header: Mini macOS/Desktop Menu & Tray Bar */}
              <div
                className="flex flex-wrap items-center justify-between gap-2 rounded-xl border px-4 py-3"
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
                    <AppWindow className="size-3.5" />
                  </span>
                  <div className="flex items-center gap-2 font-mono text-[11px]" style={{ color: 'var(--site-ink)' }}>
                    <span className="font-semibold">File</span>
                    <span style={{ color: 'var(--site-muted)' }}>Edit</span>
                    <span style={{ color: 'var(--site-muted)' }}>Window</span>
                  </div>
                </div>
                <div className="flex items-center gap-1.5 font-mono text-[10.5px]">
                  <span
                    className="rounded border px-2 py-0.5"
                    style={{
                      background: 'var(--site-card)',
                      borderColor: 'var(--site-hairline-strong)',
                      color: 'var(--site-ink)',
                    }}
                  >
                    TrayIcon
                  </span>
                  <span
                    className="rounded border px-2 py-0.5"
                    style={{
                      background: 'var(--site-card)',
                      borderColor: 'var(--site-hairline-strong)',
                      color: 'var(--site-ink)',
                    }}
                  >
                    FileDialogs
                  </span>
                </div>
              </div>

              <div className="mt-4">
                <h3
                  className="text-[19px] font-semibold tracking-[-0.015em]"
                  style={{ color: 'var(--site-ink)' }}
                >
                  Multi-Window &amp; Native Desktop Shell
                </h3>
                <p
                  className="mt-1 text-[13.5px] leading-[1.55]"
                  style={{ color: 'var(--site-muted)' }}
                >
                  Secondary windows with native vibrancy, <code className="font-mono">PlatformMenuBar</code>,{' '}
                  <code className="font-mono">PlatformTrayIcon</code>, parent-anchored file sheets, and notifications.
                </p>
              </div>
            </div>

            <div className="mt-4">
              <RustCodeBlock code={DESKTOP_CODE} />
            </div>
          </div>

          {/* Card 4: Rich Clipboard, Drag & Drop, HTTP & Files */}
          <div
            className="flex flex-col justify-between rounded-2xl border p-6"
            style={{
              background: 'var(--site-card)',
              borderColor: 'var(--site-hairline-strong)',
            }}
          >
            <div>
              {/* Visual header: Multi-MIME Drag & File Promise */}
              <div
                className="flex flex-wrap items-center justify-between gap-2 rounded-xl border px-4 py-3"
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
                    <FolderSync className="size-3.5" />
                  </span>
                  <div className="flex items-center gap-1.5 font-mono text-[11px]">
                    <span
                      className="rounded border px-1.5 py-0.5 font-medium"
                      style={{
                        background: 'var(--site-card)',
                        borderColor: 'var(--site-hairline-strong)',
                        color: 'var(--site-ink)',
                      }}
                    >
                      text/uri-list
                    </span>
                    <span
                      className="rounded border px-1.5 py-0.5 font-medium"
                      style={{
                        background: 'var(--site-card)',
                        borderColor: 'var(--site-hairline-strong)',
                        color: 'var(--site-ink)',
                      }}
                    >
                      FileSink
                    </span>
                  </div>
                </div>
                <span
                  className="inline-flex items-center gap-1 rounded-md border px-2 py-0.5 font-mono text-[10.5px]"
                  style={{
                    background: 'var(--site-card)',
                    borderColor: 'var(--site-hairline-strong)',
                    color: 'var(--site-ink)',
                  }}
                >
                  <FileUp className="size-3" style={{ color: 'var(--site-accent)' }} />
                  HttpClient + Files
                </span>
              </div>

              <div className="mt-4">
                <h3
                  className="text-[19px] font-semibold tracking-[-0.015em]"
                  style={{ color: 'var(--site-ink)' }}
                >
                  Rich Clipboard, Drag &amp; Drop, HTTP &amp; Files
                </h3>
                <p
                  className="mt-1 text-[13.5px] leading-[1.55]"
                  style={{ color: 'var(--site-muted)' }}
                >
                  Multi-MIME clipboard items, deferred file promises, external OS drag-and-drop streams, random-access{' '}
                  <code className="font-mono">Files</code>, and streaming <code className="font-mono">HttpClient</code>.
                </p>
              </div>
            </div>

            <div className="mt-4">
              <RustCodeBlock code={DATA_CODE} />
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
