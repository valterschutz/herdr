# Herdr

Herdr is a terminal-based runtime that hosts coding agents in panes, grouped into tabs and workspaces.

## Language

**Zoom**:
A tab state in which its focused pane fills the tab's whole pane area, hiding the other panes.
_Avoid_: Fullscreen (a legacy config alias for zoom, not a separate state)

**Maximize**:
A zoom whose client also hides the sidebar and tab bar, so the pane fills the whole terminal. It ends when the client stops showing that tab zoomed.
_Avoid_: Fullscreen, enhanced zoom, zen mode
