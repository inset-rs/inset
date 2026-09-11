# reveal-embedder-web/src

Host implementation. Flutter analogue: the web engine under `engine/lib/web_ui`.

## gpu.rs / platform.rs

- Change: each frame draws into a backing texture that has `COPY_SRC`, then blits 1:1 onto the canvas swapchain.
  Reason: platform — WebGPU's canvas swapchain is `RENDER_ATTACHMENT` only, and Valo's advanced blends and backdrop filters copy the target.
  Affect: a cupertino nav-bar blur or a scrolling clip still presents; drawing straight to the swapchain invalidates the command buffer.

- Change: the valo context hides missing glyphs.
  Reason: platform — Valo paints `.notdef` tofu unless asked not to.
  Affect: a character with no face yet occupies layout space and draws nothing.

## fonts.rs

- Change: the web `FontSource` fetches Roboto as the default family and Noto families for uncovered scripts, as Google Fonts CSS2 woff2 subsets keyed by the demanded `text`.
  Reason: platform — the browser has no OS font manager; Flutter web loads Noto slices from gstatic, and Valo answers misses through `FontSource` / `FontDemand`.
  Affect: the first frames have no ink for those characters until the subset arrives; then `system_fonts_changed` relayouts text. CJK is a Noto Sans SC/JP/KR chunk per demand, not a bundled TTC.
