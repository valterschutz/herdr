# Herdr

Herdr is a terminal-based runtime that hosts coding agents in panes, grouped into tabs and workspaces.

## Language

**Zoom**:
A tab state in which its focused pane fills the tab's whole pane area, hiding the other panes.
_Avoid_: Fullscreen (a legacy config alias for zoom, not a separate state)

**Maximize**:
A presentation of the focused pane whose terminal content fills the whole terminal, without persistent borders, titles, gutters, or navigation chrome. Temporary controls and notices remain available; its existing toggle and exit rules still apply.
_Avoid_: Fullscreen, enhanced zoom, zen mode
