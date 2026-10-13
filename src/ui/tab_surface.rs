use ratatui::{layout::Rect, Frame};

use super::panes::{compute_pane_infos_for_tab, render_panes, resize_tab_panes};
use crate::app::AppState;
use crate::layout::{PaneInfo, SplitBorder};
use crate::protocol::CursorState;
use crate::terminal::TerminalRuntimeRegistry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TabSurfaceTarget {
    pub(crate) workspace_index: usize,
    pub(crate) tab_index: usize,
}

/// Connection-local content geometry; it never changes ordinary tab chrome.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TabSurfacePresentation {
    #[default]
    Normal,
    Borderless,
}

impl TabSurfacePresentation {
    pub(crate) fn borderless(self, tab: &crate::workspace::Tab) -> bool {
        self == Self::Borderless && (tab.zoomed || tab.layout.pane_count() == 1)
    }
}

pub(crate) struct TabSurfaceLayout {
    pub(crate) target: Option<TabSurfaceTarget>,
    pub(crate) pane_infos: Vec<PaneInfo>,
    pub(crate) split_borders: Vec<SplitBorder>,
}

#[derive(Clone, Copy)]
pub(crate) struct TabSurfaceView<'a> {
    pub(crate) target: Option<TabSurfaceTarget>,
    pub(crate) pane_infos: &'a [PaneInfo],
    pub(crate) split_borders: &'a [SplitBorder],
}

pub(crate) fn compute_tab_surface(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    area: Rect,
    resize_panes: bool,
    cell_size: crate::kitty_graphics::HostCellSize,
) -> TabSurfaceLayout {
    let target = app.active.and_then(|workspace_index| {
        let workspace = app.workspaces.get(workspace_index)?;
        Some(TabSurfaceTarget {
            workspace_index,
            tab_index: workspace.active_tab_index(),
        })
    });
    compute_tab_surface_for(
        app,
        terminal_runtimes,
        target,
        area,
        resize_panes,
        cell_size,
    )
}

pub(crate) fn compute_tab_surface_for(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    target: Option<TabSurfaceTarget>,
    area: Rect,
    resize_panes: bool,
    cell_size: crate::kitty_graphics::HostCellSize,
) -> TabSurfaceLayout {
    compute_tab_surface_for_presentation(
        app,
        terminal_runtimes,
        target,
        area,
        resize_panes,
        cell_size,
        TabSurfacePresentation::Normal,
    )
}

pub(crate) fn compute_tab_surface_for_presentation(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    target: Option<TabSurfaceTarget>,
    area: Rect,
    resize_panes: bool,
    cell_size: crate::kitty_graphics::HostCellSize,
    presentation: TabSurfacePresentation,
) -> TabSurfaceLayout {
    let tab = target.and_then(|target| {
        app.workspaces
            .get(target.workspace_index)?
            .tabs
            .get(target.tab_index)
    });
    let split_borders = tab
        .map(|tab| {
            if tab.zoomed {
                Vec::new()
            } else {
                tab.layout.splits(area)
            }
        })
        .unwrap_or_default();
    let pane_infos = target.map_or_else(Vec::new, |target| {
        compute_pane_infos_for_tab(
            app,
            terminal_runtimes,
            target.workspace_index,
            target.tab_index,
            area,
            resize_panes,
            cell_size,
            presentation,
        )
    });

    TabSurfaceLayout {
        target,
        pane_infos,
        split_borders,
    }
}

pub(crate) fn resize_tab_surface(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    workspace_index: usize,
    tab_index: usize,
    area: Rect,
    cell_size: crate::kitty_graphics::HostCellSize,
) {
    resize_tab_surface_for_presentation(
        app,
        terminal_runtimes,
        workspace_index,
        tab_index,
        area,
        cell_size,
        TabSurfacePresentation::Normal,
    );
}

pub(crate) fn resize_tab_surface_for_presentation(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    workspace_index: usize,
    tab_index: usize,
    area: Rect,
    cell_size: crate::kitty_graphics::HostCellSize,
    presentation: TabSurfacePresentation,
) {
    let Some(tab) = app
        .workspaces
        .get(workspace_index)
        .and_then(|workspace| workspace.tabs.get(tab_index))
    else {
        return;
    };
    resize_tab_panes(
        app,
        terminal_runtimes,
        workspace_index,
        tab,
        area,
        cell_size,
        presentation,
    );
}

pub(crate) fn render_tab_surface(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    surface: TabSurfaceView<'_>,
    frame: &mut Frame,
) {
    render_panes(
        app,
        terminal_runtimes,
        frame,
        surface.target,
        surface.pane_infos,
        surface.split_borders,
    );
}

pub(crate) fn tab_surface_hyperlinks(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    surface: TabSurfaceView<'_>,
) -> Vec<((u16, u16), String, String)> {
    let Some(ws_idx) = surface.target.map(|target| target.workspace_index) else {
        return Vec::new();
    };
    if app.workspaces.get(ws_idx).is_none() {
        return Vec::new();
    }

    let mut links = Vec::new();
    for info in surface.pane_infos {
        if let Some(runtime) = app.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, info.id)
        {
            links.extend(runtime.visible_hyperlinks(info.inner_rect));
        }
    }
    links
}

pub(crate) fn tab_surface_cursor(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    surface: TabSurfaceView<'_>,
) -> Option<CursorState> {
    let ws_idx = surface.target?.workspace_index;
    let info = surface.pane_infos.iter().find(|info| info.is_focused)?;
    if !app.pane_exposes_host_cursor(ws_idx, info.id) {
        return None;
    }
    let runtime = app.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, info.id)?;
    if runtime.synchronized_output_active() {
        return None;
    }
    let scrolled_back = super::panes::pane_is_scrolled_back(runtime);
    let reveal = app.reveal_hidden_cursor_for_cjk_ime
        && (!app.cjk_ime_agent_filter_configured || {
            let detected = app
                .workspaces
                .get(ws_idx)
                .and_then(|ws| ws.terminal_id(info.id))
                .and_then(|terminal_id| app.terminals.get(terminal_id))
                .and_then(|terminal| terminal.detected_agent);
            detected.is_some_and(|agent| app.cjk_ime_agents.contains(&agent))
        });

    if let Some(cursor) = runtime.cursor_state(info.inner_rect, true) {
        let visible = if reveal {
            !scrolled_back
        } else {
            cursor.visible && !scrolled_back
        };
        Some(CursorState {
            x: cursor.x,
            y: cursor.y,
            visible,
            shape: if reveal && visible {
                app.cjk_ime_cursor_shape
            } else {
                cursor.shape
            },
        })
    } else if reveal && !scrolled_back {
        Some(CursorState {
            x: info.inner_rect.x,
            y: info.inner_rect.y,
            visible: true,
            shape: app.cjk_ime_cursor_shape,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Workspace;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Direction;
    use ratatui::Terminal;

    #[tokio::test]
    async fn borderless_surface_compute_and_resize_restore_normal_zoom_geometry() {
        use crate::config::PaneBordersConfig;
        use ratatui::widgets::Borders;
        let area = Rect::new(3, 2, 40, 12);
        let target = Some(TabSurfaceTarget {
            workspace_index: 0,
            tab_index: 0,
        });
        let runtimes = TerminalRuntimeRegistry::new();
        let cell_size = crate::kitty_graphics::HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        for multi_pane in [false, true] {
            for scrollbars in [false, true] {
                for borders in [PaneBordersConfig::Always, PaneBordersConfig::Off] {
                    let mut workspace = Workspace::test_new("borderless");
                    let focused = if multi_pane {
                        workspace.test_split(Direction::Horizontal)
                    } else {
                        workspace.tabs[0].root_pane
                    };
                    workspace.tabs[0].zoomed = multi_pane;
                    workspace.insert_test_runtime(
                        focused,
                        crate::terminal::TerminalRuntime::test_with_screen_bytes(20, 8, b"CONTENT"),
                    );
                    let mut app = AppState::test_new();
                    app.workspaces = vec![workspace];
                    app.active = Some(0);
                    app.pane_borders = borders;
                    app.pane_outer_borders = true;
                    app.pane_scrollbars = scrollbars;
                    for alternate in [false, true, false] {
                        let rt = app
                            .runtime_for_pane_in_workspace(&runtimes, 0, focused)
                            .unwrap();
                        rt.test_process_pty_bytes(if alternate {
                            b"\x1b[?1049h"
                        } else {
                            b"\x1b[?1049l"
                        });
                        let normal =
                            compute_tab_surface_for(&app, &runtimes, target, area, true, cell_size);
                        let normal_inner = normal.pane_infos[0].inner_rect;
                        let layout = compute_tab_surface_for_presentation(
                            &app,
                            &runtimes,
                            target,
                            area,
                            true,
                            cell_size,
                            TabSurfacePresentation::Borderless,
                        );
                        assert_eq!(layout.pane_infos.len(), 1);
                        let pane = &layout.pane_infos[0];
                        assert_eq!(pane.id, focused);
                        assert_eq!(pane.rect, area);
                        assert_eq!(pane.inner_rect, area);
                        assert_eq!(pane.borders, Borders::NONE);
                        assert_eq!(pane.scrollbar_rect, None);
                        assert!(layout.split_borders.is_empty());
                        assert_eq!(rt.current_size(), (12, 40));
                        assert_eq!(rt.pixel_size(), Some((320, 192)));
                        resize_tab_surface(&app, &runtimes, 0, 0, area, cell_size);
                        assert_eq!(rt.current_size(), (normal_inner.height, normal_inner.width));
                        resize_tab_surface_for_presentation(
                            &app,
                            &runtimes,
                            0,
                            0,
                            area,
                            cell_size,
                            TabSurfacePresentation::Borderless,
                        );
                        assert_eq!(rt.current_size(), (12, 40));
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn borderless_surface_does_not_flatten_prezoom_multipane_or_resize_direct_attach() {
        let mut workspace = Workspace::test_new("borderless");
        let focused = workspace.test_split(Direction::Horizontal);
        workspace.insert_test_runtime(
            focused,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(20, 8, b"LOCKED"),
        );
        let mut app = AppState::test_new();
        app.workspaces = vec![workspace];
        let terminal_id = app.workspaces[0].tabs[0]
            .terminal_id(focused)
            .unwrap()
            .clone();
        app.direct_attach_resize_locks.insert(terminal_id);
        let runtimes = TerminalRuntimeRegistry::new();
        let area = Rect::new(0, 0, 40, 12);
        let target = Some(TabSurfaceTarget {
            workspace_index: 0,
            tab_index: 0,
        });
        let cells = crate::kitty_graphics::HostCellSize::default();
        let normal = compute_tab_surface_for(&app, &runtimes, target, area, false, cells);
        let prezoom = compute_tab_surface_for_presentation(
            &app,
            &runtimes,
            target,
            area,
            true,
            cells,
            TabSurfacePresentation::Borderless,
        );
        for (normal, prezoom) in normal.pane_infos.iter().zip(&prezoom.pane_infos) {
            assert_eq!(normal.id, prezoom.id);
            assert_eq!(normal.rect, prezoom.rect);
            assert_eq!(normal.inner_rect, prezoom.inner_rect);
            assert_eq!(normal.borders, prezoom.borders);
        }
        assert_eq!(prezoom.pane_infos.len(), 2);
        assert!(!prezoom.split_borders.is_empty());
        app.workspaces[0].tabs[0].zoomed = true;
        let layout = compute_tab_surface_for_presentation(
            &app,
            &runtimes,
            target,
            area,
            true,
            cells,
            TabSurfacePresentation::Borderless,
        );
        assert_eq!(layout.pane_infos[0].inner_rect, area);
        resize_tab_surface_for_presentation(
            &app,
            &runtimes,
            0,
            0,
            area,
            cells,
            TabSurfacePresentation::Borderless,
        );
        assert_eq!(
            app.runtime_for_pane_in_workspace(&runtimes, 0, focused)
                .unwrap()
                .current_size(),
            (8, 20)
        );
    }

    #[tokio::test]
    async fn borderless_surface_render_cursor_and_hyperlinks_use_full_content_area() {
        let mut workspace = Workspace::test_new("borderless");
        let focused = workspace.tabs[0].root_pane;
        workspace.insert_test_runtime(
            focused,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(
                40,
                12,
                b"\x1b]8;;https://example.com/full\x1b\\EDGE\x1b]8;;\x1b\\",
            ),
        );
        let mut app = AppState::test_new();
        app.workspaces = vec![workspace];
        app.active = Some(0);
        let runtimes = TerminalRuntimeRegistry::new();
        let area = Rect::new(0, 0, 40, 12);
        let layout = compute_tab_surface_for_presentation(
            &app,
            &runtimes,
            Some(TabSurfaceTarget {
                workspace_index: 0,
                tab_index: 0,
            }),
            area,
            true,
            crate::kitty_graphics::HostCellSize::default(),
            TabSurfacePresentation::Borderless,
        );
        let view = TabSurfaceView {
            target: layout.target,
            pane_infos: &layout.pane_infos,
            split_borders: &layout.split_borders,
        };
        let mut terminal = Terminal::new(TestBackend::new(40, 12)).unwrap();
        terminal
            .draw(|frame| render_tab_surface(&app, &runtimes, view, frame))
            .unwrap();
        assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), "E");
        assert_eq!(terminal.backend().buffer()[(39, 11)].symbol(), " ");
        let cursor = tab_surface_cursor(&app, &runtimes, view).unwrap();
        assert_eq!((cursor.x, cursor.y), (4, 0));
        let links = tab_surface_hyperlinks(&app, &runtimes, view);
        assert!(links
            .iter()
            .any(|(point, _, uri)| *point == (0, 0) && uri == "https://example.com/full"));
    }

    #[tokio::test]
    async fn explicit_surface_layout_drives_render_cursor_and_hyperlinks() {
        let uri = "https://example.com/surface";
        let mut workspace = Workspace::test_new("shell-workspace");
        let left = workspace.tabs[0].root_pane;
        let right = workspace.test_split(Direction::Horizontal);
        workspace.insert_test_runtime(
            left,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(
                20,
                8,
                format!("\x1b]8;;{uri}\x1b\\LEFT\x1b]8;;\x1b\\").as_bytes(),
            ),
        );
        workspace.insert_test_runtime(
            right,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(20, 8, b"RIGHT"),
        );

        let mut app = AppState::test_new();
        app.workspaces = vec![workspace];
        app.active = Some(0);
        app.selected = 0;

        let full_area = Rect::new(0, 0, 106, 20);
        let area = full_area;
        let surface = compute_tab_surface(
            &app,
            &TerminalRuntimeRegistry::new(),
            area,
            false,
            crate::kitty_graphics::HostCellSize::default(),
        );
        assert_eq!(surface.pane_infos.len(), 2);
        assert!(!surface.split_borders.is_empty());

        app.view.terminal_area = Rect::new(9, 8, 7, 6);
        app.view.pane_infos.clear();

        let surface_view = TabSurfaceView {
            target: surface.target,
            pane_infos: &surface.pane_infos,
            split_borders: &surface.split_borders,
        };
        let mut terminal =
            Terminal::new(TestBackend::new(full_area.width, full_area.height)).unwrap();
        terminal
            .draw(|frame| {
                render_tab_surface(&app, &TerminalRuntimeRegistry::new(), surface_view, frame)
            })
            .unwrap();

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("LEFT"), "surface: {rendered:?}");
        assert!(rendered.contains("RIGHT"), "surface: {rendered:?}");
        assert!(!rendered.contains("shell-workspace"));

        let links = tab_surface_hyperlinks(&app, &TerminalRuntimeRegistry::new(), surface_view);
        assert!(links
            .iter()
            .any(|(_, symbol, link)| { symbol == "L" && link == uri }));
        assert!(tab_surface_cursor(&app, &TerminalRuntimeRegistry::new(), surface_view,).is_some());
    }
}
