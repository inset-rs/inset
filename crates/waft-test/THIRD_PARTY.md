# Third-party notices

FlutterTest (`waft-embedder/assets/FlutterTest.ttf`) is FlutterTest, the default font of Flutter's `flutter_test`, from Flutter's source:
https://github.com/flutter/flutter

It is the byte array `kFlutterTestFont` (2984 bytes, SHA-256 `cc25fc38471ffe455c04a09de7d569abbcb071bd1536b18bb1281a5f351bf7fe`) of `engine/src/flutter/runtime/test_font_data.cc`, at Flutter commit `ed2132410ee9`, which `engine/src/flutter/tools/gen_test_font.py` generates. `src/test_font_manager.rs` follows the engine's `txt/test_font_manager.cc` and `lib/ui/text/font_collection.cc`.

## Flutter's engine license

The engine's own license, `engine/src/flutter/LICENSE`, which governs `test_font_data.cc`:

```text
Copyright 2013 The Flutter Authors. All rights reserved.

Redistribution and use in source and binary forms, with or without modification,
are permitted provided that the following conditions are met:

    * Redistributions of source code must retain the above copyright
      notice, this list of conditions and the following disclaimer.
    * Redistributions in binary form must reproduce the above
      copyright notice, this list of conditions and the following
      disclaimer in the documentation and/or other materials provided
      with the distribution.
    * Neither the name of Google Inc. nor the names of its
      contributors may be used to endorse or promote products derived
      from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR
ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON
ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```
