# inset-embedder-web/src

Host implementation. Flutter analogue: the web engine under `engine/lib/web_ui`.

## gpu.rs / platform.rs

- Change: each frame draws into a backing texture that has `COPY_SRC`, then blits 1:1 onto the canvas swapchain.
  Reason: platform — WebGPU's canvas swapchain is `RENDER_ATTACHMENT` only, and Valo's advanced blends and backdrop filters copy the target.
  Affect: a cupertino nav-bar blur or a scrolling clip still presents; drawing straight to the swapchain invalidates the command buffer.

- Change: the valo context hides missing glyphs.
  Reason: platform — Valo paints `.notdef` tofu unless asked not to.
  Affect: a character with no face yet occupies layout space and draws nothing.

## fonts.rs / font_fallback.rs / google_fonts.rs

- Change: the interface font is compiled into the binary instead of downloaded at start-up, and no other family name is answered.
  Reason: platform — Flutter web fetches Roboto before the first frame and waits for it; shipping the same slice needs neither the wait nor the request, and Flutter web has no font by any other name either.
  Affect: the first frame has text at every weight; a font an application wants beyond that is bytes it registers, never a name fetched from the network.

- Change: a character outside that slice is drawn from one slice of the family carrying its script — Roboto for the scripts Roboto has, otherwise that script's Noto family — fetched on first use together with the family's slice list; a character no slice carries, or whose slice fails to download three times, is given up on.
  Reason: platform — Flutter ships a generated list of every Noto slice inside the engine; the same list for one family arrives with its stylesheet, so it is read when the family is first needed.
  Affect: a new script costs one stylesheet and the slices its text falls in, which the browser caches across pages, and appears a frame or two after the text; a character nothing can draw is asked for once.

- Change: the family for a script is chosen from a table of one family per script, reduced from Flutter's generated data.
  Reason: with slice lists read per family, only the family needs choosing; which slice carries a character is in the family's own list.
  Affect: a character its script's family lacks stays blank where Flutter would try further fonts; scripts other than Roboto's come in one weight, so bold text in them renders regular, as on Flutter web.

## images.rs / platform.rs

- Change: `Platform::open_image_codec` decodes with `createImageBitmap` and uploads through valo's `ImageContext` (`copyExternalImageToTexture`), then answers a one-frame codec.
  Reason: platform — the browser is the codec; valo copies an `ImageBitmap` onto the GPU without a CPU round-trip.
  Affect: PNG, JPEG, WebP, GIF and BMP the browser reads become drawable images; an animated file shows its first frame only.

## text_input.rs / platform.rs

- Change: `View` text input is a transparent DOM `input` or `textarea`, styled and placed like Flutter web's editing element, with `input` / composition / Enter forwarded as `TextEditingValue` and `TextInputAction`. While composition is in progress, `set_editing_state` does not write the DOM value or selection.
  Reason: platform — the browser IME talks to a focused HTML field, not winit's preedit events, and assigning `input.value` cancels composition.
  Affect: a field that has called `TextInput::attach` accepts typing and composition, a composing CJK cluster is not wiped by a caret update, the candidate window sits at the caret, and autofill / delta-model / a visible overlay field are absent.

## Deferred

- Animated image frames after the first. Trigger: a GIF or APNG that must play.
- Flutter web autofill form, delta model, and putting the editing element inside a view's `textEditingHost`. Trigger: AutofillGroup; `enableDeltaModel`.
