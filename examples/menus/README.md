# menus

The system's menus through Inset's widgets, in one window:

- `PlatformMenuBar`:
  - the application menu with the provided About, Hide and Quit;
  - File, whose New has ⌘N;
  - View, whose Show Hints checkbox follows the app's state;
  - the provided Edit items, which act on the text field.
- `PlatformTrayIcon`: a template image with a tooltip, whose menu shows the window or quits. The menu opens on each system's own trigger: macOS on a click, a Linux desktop when it chooses, Windows on a right click. A click the system reports, on Windows and Linux, brings the window forward; macOS reports none for an icon with a menu.
- The system's own context menu on a right click in the window.

The window shows the last choice. Each place is the system's or none: where the platform has none (`Platform::menu_bar()`, `tray_icons()` or `popup_menus()` answer `None`), the window says so in its place, where an app would draw its own. Windows and Linux have no menu bar, and Linux no context menu.

## Run

From the repository root:

```sh
cargo run -p menus
```
