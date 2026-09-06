use super::*;
use ratatui::style::Color;

fn restore_mode_bar(
    frame: &mut FrameData,
    bar: Option<Rect>,
    cells: Option<&[crate::protocol::CellData]>,
) {
    let (Some(bar), Some(cells)) = (bar, cells) else {
        return;
    };
    let start = usize::from(bar.y) * usize::from(frame.width) + usize::from(bar.x);
    frame.cells[start..start + usize::from(bar.width)].clone_from_slice(cells);
    if frame
        .cursor
        .as_ref()
        .is_some_and(|cursor| cursor.y == bar.y)
    {
        frame.cursor = None;
    }
}

pub(super) fn hover_background(palette: &Palette, focused: bool) -> ratatui::style::Color {
    let _ = focused;
    // Blend toward the sidebar, keeping hover much subtler than selection.
    let base = if palette.sidebar_bg == Color::Reset {
        palette.panel_bg
    } else {
        palette.sidebar_bg
    };
    match (base, palette.active_row_bg) {
        (Color::Rgb(r, g, b), Color::Rgb(ar, ag, ab)) => {
            let blend =
                |base: u8, active: u8| ((u16::from(base) * 3 + u16::from(active)) / 4) as u8;
            Color::Rgb(blend(r, ar), blend(g, ag), blend(b, ab))
        }
        _ => palette.surface0,
    }
}

fn set_background(buffer: &mut Buffer, rect: Rect, color: ratatui::style::Color) {
    for y in rect.y..rect.bottom() {
        for x in rect.x..rect.right() {
            if let Some(cell) = buffer.cell_mut((x, y)) {
                cell.set_bg(color);
            }
        }
    }
}

fn workspace_hover_rect(
    hits: &ShellHitMap,
    endpoint_id: &ClientEndpointId,
    workspace_id: &str,
) -> Option<Rect> {
    hits.workspaces
        .iter()
        .find(|hit| &hit.endpoint_id == endpoint_id && hit.workspace_id == workspace_id)
        .map(|hit| hit.rect)
}

fn agent_hover_rect(
    hits: &ShellHitMap,
    endpoint_id: &ClientEndpointId,
    pane_id: &str,
) -> Option<Rect> {
    hits.agents
        .iter()
        .find(|(_, candidate)| candidate == pane_id)
        .map(|(rect, _)| *rect)
        .or_else(|| {
            hits.endpoint_agents
                .iter()
                .find(|(_, endpoint, candidate)| endpoint == endpoint_id && candidate == pane_id)
                .map(|(rect, _, _)| *rect)
        })
}

fn pane_label_rect(
    hits: &ShellHitMap,
    snapshot: &ClientShellSnapshot,
    pane_id: &str,
    app_handle: bool,
) -> Option<Rect> {
    let hit = hits
        .panes
        .iter()
        .find(|hit| !hit.popup && hit.pane_id == pane_id && hit.rect.y < hit.inner_rect.y)?;
    if app_handle {
        return pane_drag_handle_rect(hit);
    }
    let label = snapshot
        .panes
        .iter()
        .find(|pane| pane.pane_id == pane_id)?
        .label
        .as_deref()?
        .trim();
    if label.is_empty() || hit.rect.width <= 4 {
        return None;
    }
    let width = unicode_width::UnicodeWidthStr::width(label)
        .saturating_add(4)
        .min(hit.rect.width.saturating_sub(2) as usize) as u16;
    Some(Rect::new(
        hit.rect.x.saturating_add(1),
        hit.rect.y,
        width,
        1,
    ))
}

pub(super) fn pane_drag_handle_rect(hit: &PaneHit) -> Option<Rect> {
    (!hit.popup && hit.rect.y < hit.inner_rect.y && hit.rect.width > 6)
        .then(|| Rect::new(hit.rect.x + 1, hit.rect.y, 3, 1))
}

pub(super) fn pane_header_label(
    snapshot: &ClientShellSnapshot,
    surface: &PaneSurfaceFrame,
    pane_id: &str,
) -> Option<String> {
    if let Some(label) = snapshot
        .panes
        .iter()
        .find(|pane| pane.pane_id == pane_id)
        .and_then(|pane| pane.label.as_deref())
        .map(str::trim)
        .filter(|label| !label.is_empty())
    {
        return Some(label.to_owned());
    }
    // Older servers only send dynamic titles in the rendered border. Reuse
    // that visible name rather than inventing a different name when detached.
    let pane = surface.panes.iter().find(|pane| pane.pane_id == pane_id)?;
    if pane.rect.y >= pane.inner_rect.y {
        return None;
    }
    let mut label = String::new();
    for x in pane.rect.x.saturating_add(1)
        ..pane
            .rect
            .x
            .saturating_add(pane.rect.width)
            .saturating_sub(1)
    {
        let index = usize::from(pane.rect.y) * usize::from(surface.frame.width) + usize::from(x);
        if let Some(cell) = surface.frame.cells.get(index) {
            label.push_str(&cell.symbol);
        }
    }
    let label = label.trim_matches(|c: char| c.is_whitespace() || "─━═┌┐╭╮┬┤├┴⠿⋮⠇".contains(c));
    (!label.is_empty()).then(|| label.to_owned())
}

impl ClientShellState {
    fn compose_unavailable(&mut self, cols: u16, rows: u16) -> FrameData {
        let layout = self.layout(cols, rows);
        let mut buffer = Buffer::empty(Rect::new(0, 0, cols, rows));
        buffer.set_style(
            buffer.area,
            Style::default()
                .fg(self.config.palette.text)
                .bg(self.config.palette.panel_bg),
        );
        self.hits = ShellHitMap::default();
        let sidebar = if layout.sidebar.width > 0 {
            layout.sidebar
        } else {
            Rect::new(0, 1, cols, rows.saturating_sub(2))
        };
        super::endpoint_sidebar::render_expanded(
            &mut buffer,
            sidebar,
            self.snapshot.as_deref(),
            &self.config,
            &mut render::ShellRenderState {
                endpoints: &self.endpoints,
                active_endpoint_id: &self.active_endpoint_id,
                collapsed_endpoints: &self.collapsed_endpoints,
                collapsed_groups: &self.collapsed_groups,
                workspace_scroll: &mut self.workspace_scroll,
                agent_scroll: &mut self.agent_scroll,
                tab_scroll: &mut self.tab_scroll,
                reveal_focused_workspace: &mut self.reveal_focused_workspace,
                reveal_focused_tab: &mut self.reveal_focused_tab,
                sidebar_collapsed: false,
                sidebar_section_split: self.sidebar_section_split,
                tab_drag_insert_index: None,
                selected_workspace_id: self.navigate_workspace_id.as_deref(),
                dragged_workspace_id: None,
                workspace_drop_indicator_row: None,
            },
            &mut self.hits,
        );
        if !self.config.mouse_capture {
            self.hits = ShellHitMap::default();
        }
        let message = self.endpoint_error.clone().unwrap_or_else(|| {
            let status = self
                .endpoint_status(&self.active_endpoint_id)
                .unwrap_or(ClientEndpointStatus::Connecting);
            let (_, label, _) = endpoint_status_presentation(status, &self.config.palette);
            format!(
                "{}: {label}. Select a connected machine.",
                self.active_endpoint_label()
            )
        });
        let message_area = if layout.sidebar.width > 0 {
            layout.pane_surface
        } else {
            Rect::new(0, 0, cols, 1)
        };
        render::put_text(
            &mut buffer,
            message_area.x,
            message_area.y,
            message_area.width,
            &message,
            Style::default().fg(self.config.palette.overlay0),
        );
        render::render_mode_bar(
            &mut buffer,
            Rect::new(0, 0, cols, rows),
            self.mode,
            None,
            self.endpoint_error.as_deref(),
            false,
            &self.config.keybinds,
            &self.config.palette,
        );
        FrameData::from_ratatui_buffer_with_hyperlinks(&buffer, None, &[])
    }

    pub(crate) fn compose(&mut self, cols: u16, rows: u16) -> Option<FrameData> {
        self.last_composed_size = Some((cols, rows));
        if self.snapshot.is_none() || self.pane_surface.is_none() {
            return Some(self.compose_unavailable(cols, rows));
        }
        let snapshot = self.snapshot.as_deref()?;
        // A one-step successor is retained separately until its exact snapshot arrives; do not
        // keep composing the now-superseded current pair while it is pending.
        if self.pending_pane_surface.is_some() {
            return None;
        }
        let surface = self.pane_surface.as_ref()?;
        if snapshot.revision != surface.projection_revision {
            return None;
        }
        let layout = self.layout(cols, rows);
        if self.last_tab_bar_width != Some(layout.tab_bar.width) {
            self.last_tab_bar_width = Some(layout.tab_bar.width);
            self.reveal_focused_tab = true;
        }
        let tab_drag_insert_index = match &self.chrome_drag {
            Some(ClientChromeDrag::Tab { insert_index, .. }) => *insert_index,
            _ => None,
        };
        let (dragged_workspace_id, workspace_drop_indicator_row) = match &self.chrome_drag {
            Some(ClientChromeDrag::Workspace {
                source_workspace_id,
                target,
            }) => (
                Some(source_workspace_id.as_str()),
                target.as_ref().map(|(_, row)| *row),
            ),
            _ => (None, None),
        };
        let mut buffer = Buffer::empty(Rect::new(0, 0, cols, rows));
        if self.config.top_padding > 0 {
            buffer.set_style(
                Rect::new(0, 0, cols, self.config.top_padding.min(rows)),
                Style::default().bg(self.config.palette.panel_bg),
            );
        }
        self.hits = render::render_shell(
            &mut buffer,
            layout,
            snapshot,
            &self.config,
            render::ShellRenderState {
                endpoints: &self.endpoints,
                active_endpoint_id: &self.active_endpoint_id,
                collapsed_endpoints: &self.collapsed_endpoints,
                collapsed_groups: &self.collapsed_groups,
                workspace_scroll: &mut self.workspace_scroll,
                agent_scroll: &mut self.agent_scroll,
                tab_scroll: &mut self.tab_scroll,
                reveal_focused_workspace: &mut self.reveal_focused_workspace,
                reveal_focused_tab: &mut self.reveal_focused_tab,
                sidebar_collapsed: self.sidebar_collapsed,
                sidebar_section_split: self.sidebar_section_split,
                tab_drag_insert_index,
                selected_workspace_id: (self.mode == ClientShellMode::Navigate)
                    .then_some(self.navigate_workspace_id.as_deref())
                    .flatten(),
                dragged_workspace_id,
                workspace_drop_indicator_row,
            },
        );
        self.hits.panes = surface
            .panes
            .iter()
            .map(|pane| PaneHit {
                rect: Rect::new(
                    layout.pane_surface.x.saturating_add(pane.rect.x),
                    layout.pane_surface.y.saturating_add(pane.rect.y),
                    pane.rect.width,
                    pane.rect.height,
                ),
                inner_rect: Rect::new(
                    layout.pane_surface.x.saturating_add(pane.inner_rect.x),
                    layout.pane_surface.y.saturating_add(pane.inner_rect.y),
                    pane.inner_rect.width,
                    pane.inner_rect.height,
                ),
                scrollbar_rect: pane.scrollbar_rect.map(|rect| {
                    Rect::new(
                        layout.pane_surface.x.saturating_add(rect.x),
                        layout.pane_surface.y.saturating_add(rect.y),
                        rect.width,
                        rect.height,
                    )
                }),
                scroll: pane.scroll.map(|metrics| crate::pane::ScrollMetrics {
                    offset_from_bottom: usize::try_from(metrics.offset_from_bottom)
                        .unwrap_or(usize::MAX),
                    max_offset_from_bottom: usize::try_from(metrics.max_offset_from_bottom)
                        .unwrap_or(usize::MAX),
                    viewport_rows: usize::try_from(metrics.viewport_rows).unwrap_or(usize::MAX),
                }),
                pane_id: pane.pane_id.clone(),
                popup: false,
                mouse_reporting: pane.mouse_reporting,
                sgr_pixel_mouse: pane.sgr_pixel_mouse,
                pixel_width: pane.pixel_width,
                pixel_height: pane.pixel_height,
            })
            .collect();
        let topology_signature = pane_surface_topology_signature(surface);
        self.hits.pane_splits = surface
            .splits
            .iter()
            .map(|split| PaneSplitHit {
                direction: split.direction,
                pos: match split.direction {
                    crate::protocol::PaneSurfaceSplitDirection::Horizontal => {
                        layout.pane_surface.x.saturating_add(split.pos)
                    }
                    crate::protocol::PaneSurfaceSplitDirection::Vertical => {
                        layout.pane_surface.y.saturating_add(split.pos)
                    }
                },
                area: Rect::new(
                    layout.pane_surface.x.saturating_add(split.area.x),
                    layout.pane_surface.y.saturating_add(split.area.y),
                    split.area.width,
                    split.area.height,
                ),
                hit_rect: Rect::new(
                    layout.pane_surface.x.saturating_add(split.hit_rect.x),
                    layout.pane_surface.y.saturating_add(split.hit_rect.y),
                    split.hit_rect.width,
                    split.hit_rect.height,
                ),
                path: split.path.clone(),
                topology_signature,
            })
            .collect();
        if !self.config.mouse_capture {
            self.hits.pane_splits.clear();
        }
        let mode_bar_area = if layout.mobile_header.is_empty()
            && self.config.tab_bar_position == TabBarPositionConfig::Bottom
            && !layout.tab_bar.is_empty()
        {
            layout.tab_bar
        } else {
            layout.pane_surface
        };
        let mobile_navigate_panel = !layout.mobile_header.is_empty()
            && self.mode == ClientShellMode::Navigate
            && self.endpoint_error.is_none();
        let mode_bar = if mobile_navigate_panel || self.overlay.is_some() {
            None
        } else {
            render::render_mode_bar(
                &mut buffer,
                mode_bar_area,
                self.mode,
                self.copy_mode.as_ref(),
                self.endpoint_error.as_deref(),
                snapshot.update_available.is_some(),
                &self.config.keybinds,
                &self.config.palette,
            )
        };
        if mode_bar == Some(layout.tab_bar) {
            self.hits.tabs.clear();
            self.hits.new_tab = Rect::default();
            self.hits.tab_scroll_left = Rect::default();
            self.hits.tab_scroll_right = Rect::default();
        }
        if let Some(workspace_id) = self.hovered_workspace_id.as_deref() {
            let selected = self.mode == ClientShellMode::Navigate
                && self.navigate_workspace_id.as_deref() == Some(workspace_id);
            let focused = snapshot
                .workspaces
                .iter()
                .any(|workspace| workspace.workspace_id == workspace_id && workspace.focused);
            if !selected && !focused {
                if let Some(rect) =
                    workspace_hover_rect(&self.hits, &self.active_endpoint_id, workspace_id)
                {
                    set_background(
                        &mut buffer,
                        rect,
                        hover_background(&self.config.palette, focused),
                    );
                }
            }
        }
        if let Some(pane_id) = self.hovered_agent_pane_id.as_deref() {
            let focused = snapshot
                .agents
                .iter()
                .any(|agent| agent.pane_id == pane_id && agent.focused);
            if let Some(rect) =
                agent_hover_rect(&self.hits, &self.active_endpoint_id, pane_id).filter(|_| !focused)
            {
                set_background(
                    &mut buffer,
                    rect,
                    hover_background(&self.config.palette, focused),
                );
            }
        }
        if let Some(key) = self.hovered_agent_group_key.as_deref() {
            if let Some(hit) = self
                .hits
                .agent_groups
                .iter()
                .find(|hit| hit.key == key && !hit.focused)
            {
                set_background(
                    &mut buffer,
                    hit.rect,
                    hover_background(&self.config.palette, hit.focused),
                );
            }
        }
        if self.hovered_new_workspace {
            set_background(
                &mut buffer,
                self.hits.new_workspace,
                hover_background(&self.config.palette, false),
            );
        }
        if self.hovered_sleep {
            set_background(
                &mut buffer,
                self.hits.sleep_button,
                hover_background(&self.config.palette, false),
            );
        }
        if self.hovered_global_launcher {
            set_background(
                &mut buffer,
                self.hits.global_launcher,
                hover_background(&self.config.palette, false),
            );
        }
        if let Some(tab_id) = self.hovered_tab_id.as_deref() {
            if let Some((rect, _)) = self
                .hits
                .tabs
                .iter()
                .find(|(_, candidate)| candidate == tab_id)
            {
                let focused = snapshot
                    .tabs
                    .iter()
                    .any(|tab| tab.tab_id == tab_id && tab.focused);
                if !focused {
                    set_background(
                        &mut buffer,
                        *rect,
                        hover_background(&self.config.palette, focused),
                    );
                }
            }
        }
        let mut frame = FrameData::from_ratatui_buffer_with_hyperlinks(&buffer, None, &[]);
        let mode_bar_cells = mode_bar.map(|bar| {
            let start = usize::from(bar.y) * usize::from(frame.width) + usize::from(bar.x);
            frame.cells[start..start + usize::from(bar.width)].to_vec()
        });
        blit_pane_surface(&mut frame, &surface.frame, layout.pane_surface);
        if self.config.top_padding > 0 && self.config.mouse_capture {
            let cursor = frame.cursor.clone();
            let mut composed = frame.to_ratatui_buffer()?;
            for hit in &self.hits.panes {
                let Some(handle) = pane_drag_handle_rect(hit) else {
                    continue;
                };
                let style = Style::default().fg(self.config.palette.text);
                let label = pane_header_label(snapshot, surface, &hit.pane_id).unwrap_or_default();
                // Replace the old header, including any grip painted by an
                // older server. The client owns exactly one grip + title.
                for x in hit.rect.x + 1..hit.rect.right() - 1 {
                    if let Some(cell) = composed.cell_mut((x, hit.rect.y)) {
                        cell.set_symbol("─");
                    }
                }
                composed.set_stringn(
                    handle.x,
                    handle.y,
                    format!(" ⠿ {label} "),
                    usize::from(hit.rect.width - 2),
                    style,
                );
            }
            frame.replace_from_ratatui_buffer_preserving_effects(&composed, cursor);
        }
        restore_mode_bar(&mut frame, mode_bar, mode_bar_cells.as_deref());
        if let Some(pane_id) = self.hovered_pane_label_id.as_deref() {
            if let Some(rect) =
                pane_label_rect(&self.hits, snapshot, pane_id, self.config.top_padding > 0)
            {
                let focused = snapshot
                    .panes
                    .iter()
                    .any(|pane| pane.pane_id == pane_id && pane.focused);
                let cursor = frame.cursor.clone();
                if let Some(mut composed) = frame.to_ratatui_buffer().filter(|_| !focused) {
                    set_background(
                        &mut composed,
                        rect,
                        hover_background(&self.config.palette, focused),
                    );
                    frame.replace_from_ratatui_buffer_preserving_effects(&composed, cursor);
                }
            }
        }
        let has_selection = self
            .selection
            .as_ref()
            .is_some_and(|selection| selection.is_visible());
        let has_search = self
            .copy_mode
            .as_ref()
            .is_some_and(|copy_mode| !copy_mode.search_matches.is_empty());
        if has_selection || has_search {
            let cursor = frame.cursor.clone();
            let mut composed = frame.to_ratatui_buffer()?;
            for hit in &self.hits.panes {
                let copy_surface_coherent =
                    client_copy_surface_coherent(self.copy_mode.as_ref(), hit);
                if copy_surface_coherent {
                    render_client_copy_search_highlights(
                        &mut composed,
                        self.copy_mode.as_ref(),
                        hit,
                        &self.config.palette,
                        false,
                    );
                }
                let selection_is_stale_copy_projection = !copy_surface_coherent
                    && self.copy_mode.as_ref().is_some_and(|copy_mode| {
                        copy_mode.pane_id == hit.pane_id
                            && self
                                .selection
                                .as_ref()
                                .is_some_and(|selection| selection.pane_id == hit.pane_id)
                    });
                if !selection_is_stale_copy_projection {
                    crate::ui::render_selection_highlight(
                        self.selection.as_ref(),
                        &mut composed,
                        &hit.pane_id,
                        hit.inner_rect,
                        hit.scroll,
                        &self.config.palette,
                        crate::terminal_theme::TerminalTheme::default(),
                    );
                }
                if copy_surface_coherent {
                    render_client_copy_search_highlights(
                        &mut composed,
                        self.copy_mode.as_ref(),
                        hit,
                        &self.config.palette,
                        true,
                    );
                }
            }
            frame.replace_from_ratatui_buffer_preserving_effects(&composed, cursor);
        }
        let pane_drop_target = match self.chrome_drag.as_ref() {
            Some(ClientChromeDrag::Tab {
                pane_drop_target, ..
            })
            | Some(ClientChromeDrag::Pane {
                pane_drop_target, ..
            }) => pane_drop_target.as_ref(),
            _ => None,
        };
        if let Some(target) = pane_drop_target {
            let rect = target.highlight_rect();
            if rect.width > 0 && rect.height > 0 {
                let cursor = frame.cursor.clone();
                let mut composed = frame.to_ratatui_buffer()?;
                let right = rect
                    .right()
                    .saturating_sub(1)
                    .min(frame.width.saturating_sub(1));
                let bottom = rect
                    .bottom()
                    .saturating_sub(1)
                    .min(frame.height.saturating_sub(1));
                for y in rect.y..=bottom {
                    for x in rect.x..=right {
                        if x == rect.x || x == right || y == rect.y || y == bottom {
                            let base = match composed[(x, y)].bg {
                                Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
                                _ => self.config.palette.panel_bg,
                            };
                            let color = match (base, self.config.palette.accent) {
                                (Color::Rgb(r, g, b), Color::Rgb(ar, ag, ab)) => {
                                    let mix = |base: u8, accent: u8| {
                                        ((u16::from(base) * 7 + u16::from(accent)) / 8) as u8
                                    };
                                    Color::Rgb(mix(r, ar), mix(g, ag), mix(b, ab))
                                }
                                _ => self.config.palette.surface0,
                            };
                            composed[(x, y)].set_bg(color);
                        }
                    }
                }
                frame.replace_from_ratatui_buffer_preserving_effects(&composed, cursor);
            }
        }
        if (matches!(
            self.chrome_drag.as_ref(),
            Some(ClientChromeDrag::Pane { .. })
        ) || self.agent_press.is_some())
            && !self.hits.new_tab.is_empty()
        {
            let cursor = frame.cursor.clone();
            let mut composed = frame.to_ratatui_buffer()?;
            composed.set_style(
                self.hits.new_tab,
                Style::default().bg(hover_background(&self.config.palette, false)),
            );
            frame.replace_from_ratatui_buffer_preserving_effects(&composed, cursor);
        }
        if self.mode == ClientShellMode::Copy {
            frame.cursor = None;
            if let Some(copy_mode) = self.copy_mode.as_ref() {
                if let Some(hit) = self.hits.panes.iter().find(|hit| {
                    hit.pane_id == copy_mode.pane_id
                        && client_copy_surface_coherent(Some(copy_mode), hit)
                }) {
                    let viewport_top = copy_mode
                        .max_offset_from_bottom
                        .saturating_sub(copy_mode.offset_from_bottom)
                        .min(u32::MAX as usize) as u32;
                    let viewport_row = copy_mode.cursor.row.saturating_sub(viewport_top);
                    if viewport_row < u32::from(hit.inner_rect.height)
                        && copy_mode.cursor.col < hit.inner_rect.width
                    {
                        let mut composed = frame.to_ratatui_buffer()?;
                        let x = hit.inner_rect.x + copy_mode.cursor.col;
                        let y = hit.inner_rect.y + viewport_row as u16;
                        composed[(x, y)].set_style(
                            Style::default()
                                .fg(match self.config.palette.panel_bg {
                                    ratatui::style::Color::Reset => self.config.palette.surface_dim,
                                    color => color,
                                })
                                .bg(self.config.palette.accent)
                                .add_modifier(Modifier::BOLD),
                        );
                        frame.replace_from_ratatui_buffer_preserving_effects(&composed, None);
                    } else {
                        frame.cursor = None;
                    }
                }
            }
        }
        restore_mode_bar(&mut frame, mode_bar, mode_bar_cells.as_deref());
        self.hits.notification_toast = Rect::default();
        let has_config_diagnostic = self.config_diagnostic.is_some();
        let active_lifecycle = self
            .endpoints
            .iter()
            .find(|endpoint| endpoint.endpoint_id == self.active_endpoint_id)
            .filter(|endpoint| endpoint.status != ClientEndpointStatus::Online)
            .map(|endpoint| (endpoint.label.clone(), endpoint.status));
        if has_config_diagnostic
            || active_lifecycle.is_some()
            || self.visible_endpoint_notice.is_some()
            || self.visible_notification.is_some()
        {
            let cursor = frame.cursor.clone();
            let mut composed = frame.to_ratatui_buffer()?;
            if let Some(diagnostic) = self.config_diagnostic.as_deref() {
                let diagnostic_area = if layout.mobile_header.is_empty() {
                    Rect::new(0, 0, cols, rows)
                } else {
                    layout.pane_surface
                };
                crate::ui::render_config_diagnostic_buffer(
                    &mut composed,
                    diagnostic_area,
                    diagnostic,
                    &self.config.palette,
                );
            }
            let lifecycle_offset = active_lifecycle.as_ref().map_or(0, |(label, status)| {
                let _ = endpoint_notices::render_lifecycle_banner(
                    &mut composed,
                    Rect::new(0, 0, cols, rows),
                    label,
                    *status,
                    u16::from(has_config_diagnostic) + layout.mobile_header.height,
                    &self.config.palette,
                );
                1
            });
            if let Some(notice) = self.visible_endpoint_notice.as_ref() {
                self.hits.notification_toast = if layout.mobile_header.is_empty() {
                    endpoint_notices::render_notice(
                        &mut composed,
                        Rect::new(0, 0, cols, rows),
                        notice,
                        u16::from(has_config_diagnostic) + lifecycle_offset,
                        &self.config.palette,
                    )
                } else {
                    endpoint_notices::render_mobile_banner(
                        &mut composed,
                        Rect::new(0, 0, cols, rows),
                        notice,
                        has_config_diagnostic || lifecycle_offset > 0,
                        &self.config.palette,
                    )
                };
            } else if let Some(notification) = self.visible_notification.as_ref() {
                self.hits.notification_toast = if layout.mobile_header.is_empty() {
                    notifications::render_visible_notification(
                        &mut composed,
                        Rect::new(0, 0, cols, rows),
                        notification,
                        self.config.toast_position,
                        u16::from(has_config_diagnostic) + lifecycle_offset,
                        &self.config.palette,
                    )
                } else {
                    notifications::render_mobile_notification_banner(
                        &mut composed,
                        Rect::new(0, 0, cols, rows),
                        notification,
                        has_config_diagnostic || lifecycle_offset > 0,
                        &self.config.palette,
                    )
                };
            }
            frame.replace_from_ratatui_buffer_preserving_effects(&composed, cursor);
        }
        if let Some(feedback) = self.copy_feedback.as_ref() {
            let cursor = frame.cursor.clone();
            let mut composed = frame.to_ratatui_buffer()?;
            let base_offset = u16::from(has_config_diagnostic);
            let feedback_area = if layout.mobile_header.is_empty() {
                layout.pane_surface
            } else {
                Rect::new(0, 0, cols, rows)
            };
            let offset = crate::ui::copy_feedback_offset_for_toast(
                feedback_area,
                feedback,
                base_offset,
                self.config.clipboard_toast_position,
                self.hits.notification_toast,
            );
            crate::ui::render_copy_feedback_buffer(
                &mut composed,
                feedback_area,
                feedback,
                offset,
                self.config.clipboard_toast_position,
                &self.config.palette,
            );
            frame.replace_from_ratatui_buffer_preserving_effects(&composed, cursor);
        }
        self.hits.popup = None;
        if let Some(popup) = surface.popup.as_deref() {
            let width = popup.width.map(client_popup_size);
            let height = popup.height.map(client_popup_size);
            if let Some(geometry) =
                crate::popup_size::resolve_popup_geometry(width, height, layout.pane_surface)
            {
                let mut composed = frame.to_ratatui_buffer()?;
                let block = ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(ratatui::style::Style::default().fg(self.config.palette.accent))
                    .title(popup.title.clone())
                    .style(ratatui::style::Style::default().bg(self.config.palette.panel_bg));
                ratatui::widgets::Widget::render(
                    ratatui::widgets::Clear,
                    geometry.outer,
                    &mut composed,
                );
                ratatui::widgets::Widget::render(block, geometry.outer, &mut composed);
                frame.replace_from_ratatui_buffer_preserving_effects(&composed, None);
                blit_pane_surface(&mut frame, &popup.frame, geometry.inner);
                self.hits.popup = Some(PaneHit {
                    rect: geometry.outer,
                    inner_rect: geometry.inner,
                    scrollbar_rect: None,
                    scroll: None,
                    pane_id: popup.terminal_id.clone(),
                    popup: true,
                    mouse_reporting: popup.mouse_reporting,
                    sgr_pixel_mouse: popup.sgr_pixel_mouse,
                    pixel_width: popup.pixel_width,
                    pixel_height: popup.pixel_height,
                });
            }
        }
        if !layout.mobile_header.is_empty()
            && self.mode == ClientShellMode::Navigate
            && self.overlay.is_none()
        {
            let mut composed = frame.to_ratatui_buffer()?;
            super::mobile::render_mobile_switcher(
                &mut composed,
                Rect::new(0, 0, cols, rows),
                snapshot,
                &self.endpoints,
                &self.active_endpoint_id,
                &self.config,
                self.navigate_workspace_id.as_deref(),
                &mut self.mobile_switcher_scroll,
                &mut self.reveal_mobile_workspace,
                &mut self.hits,
            );
            if let Some((label, status)) = active_lifecycle.as_ref() {
                let _ = endpoint_notices::render_lifecycle_banner(
                    &mut composed,
                    Rect::new(0, 0, cols, rows),
                    label,
                    *status,
                    2,
                    &self.config.palette,
                );
            }
            if let Some(notice) = self.visible_endpoint_notice.as_ref() {
                self.hits.notification_toast = endpoint_notices::render_mobile_banner(
                    &mut composed,
                    Rect::new(0, 0, cols, rows),
                    notice,
                    active_lifecycle.is_some(),
                    &self.config.palette,
                );
            }
            frame.replace_from_ratatui_buffer_preserving_effects(&composed, None);
            self.hits.panes.clear();
            self.hits.pane_splits.clear();
            self.hits.popup = None;
        }
        restore_mode_bar(&mut frame, mode_bar, mode_bar_cells.as_deref());
        if let Some(overlay) = self.overlay.as_ref() {
            let mut composed = frame.to_ratatui_buffer()?;
            let cursor = if let ClientShellOverlay::ContextMenu(menu) = overlay {
                self.hits.context_menu_rows =
                    render::render_context_menu(&mut composed, menu, &self.config.palette)?;
                None
            } else if let ClientShellOverlay::GlobalMenu(menu) = overlay {
                self.hits.global_menu_rows = render::render_global_menu(
                    &mut composed,
                    self.hits.global_launcher,
                    menu,
                    snapshot,
                    &self.config.palette,
                )?;
                None
            } else {
                let rendered = render::render_client_overlay(
                    &mut composed,
                    overlay,
                    snapshot,
                    &self.endpoints,
                    &self.active_endpoint_id,
                    &self.config.keybinds,
                    &self.config.palette,
                )?;
                self.hits.overlay_primary = rendered.primary;
                self.hits.overlay_clear = rendered.clear;
                self.hits.overlay_cancel = rendered.cancel;
                self.hits.navigator_popup = rendered.navigator_popup;
                self.hits.navigator_search = rendered.navigator_search;
                self.hits.navigator_rows = rendered.navigator_rows;
                self.hits.worktree_search = rendered.worktree_search;
                self.hits.worktree_rows = rendered.worktree_rows;
                self.hits.help_popup = rendered.help_popup;
                self.hits.help_scrollbar = rendered.help_scrollbar;
                self.hits.help_scroll_metrics = rendered.help_scroll_metrics;
                self.hits.help_max_scroll = rendered.help_max_scroll;
                self.hits.settings_popup = rendered.settings_popup;
                self.hits.settings_tabs = rendered.settings_tabs;
                self.hits.settings_choices = rendered.settings_choices;
                self.hits.product_announcement_scrollbar = rendered.product_announcement_scrollbar;
                self.hits.product_announcement_scroll_metrics =
                    rendered.product_announcement_scroll_metrics;
                self.hits.product_announcement_max_scroll =
                    rendered.product_announcement_max_scroll;
                self.hits.release_notes_scrollbar = rendered.release_notes_scrollbar;
                self.hits.release_notes_scroll_metrics = rendered.release_notes_scroll_metrics;
                self.hits.release_notes_max_scroll = rendered.release_notes_max_scroll;
                rendered.cursor
            };
            frame.replace_from_ratatui_buffer_preserving_effects(&composed, cursor);
        }
        if let Some(ClientShellOverlay::Help(help)) = self.overlay.as_mut() {
            help.scroll = help.scroll.min(self.hits.help_max_scroll);
        }
        if let Some(ClientShellOverlay::ProductAnnouncement(announcement)) = self.overlay.as_mut() {
            announcement.scroll = announcement
                .scroll
                .min(u16::try_from(self.hits.product_announcement_max_scroll).unwrap_or(u16::MAX));
        }
        if let Some(ClientShellOverlay::ReleaseNotes(notes)) = self.overlay.as_mut() {
            notes.scroll = notes
                .scroll
                .min(u16::try_from(self.hits.release_notes_max_scroll).unwrap_or(u16::MAX));
        }
        if self.sleep_started.is_some() {
            let mut buffer = frame.to_ratatui_buffer()?;
            render::render_sleep_overlay(
                &mut buffer,
                &self.sleep_server,
                self.sleep_frame,
                self.sleep_remaining,
                self.sleep_phrase,
                snapshot,
                &self.config.palette,
            );
            frame.replace_from_ratatui_buffer_preserving_effects(&buffer, None);
        }
        if self.endpoint_status(&self.active_endpoint_id) != Some(ClientEndpointStatus::Online) {
            frame.cursor = None;
            self.hits.panes.clear();
            self.hits.pane_splits.clear();
            self.hits.popup = None;
        }
        self.compose_graphics(&mut frame, layout);
        Some(frame)
    }
}

fn client_copy_surface_coherent(copy_mode: Option<&ClientCopyModeState>, hit: &PaneHit) -> bool {
    copy_mode
        .filter(|copy_mode| copy_mode.pane_id == hit.pane_id)
        .is_none_or(|copy_mode| {
            copy_mode.geometry == (hit.inner_rect.width, hit.inner_rect.height)
                && hit.scroll.is_some_and(|scroll| {
                    scroll.offset_from_bottom == copy_mode.offset_from_bottom
                        && scroll.max_offset_from_bottom == copy_mode.max_offset_from_bottom
                })
        })
}

fn render_client_copy_search_highlights(
    buffer: &mut Buffer,
    copy_mode: Option<&ClientCopyModeState>,
    hit: &PaneHit,
    palette: &Palette,
    current_only: bool,
) {
    let Some(copy_mode) = copy_mode.filter(|copy_mode| copy_mode.pane_id == hit.pane_id) else {
        return;
    };
    if hit.inner_rect.is_empty() {
        return;
    }
    let top = copy_mode
        .max_offset_from_bottom
        .saturating_sub(copy_mode.offset_from_bottom)
        .min(u32::MAX as usize) as u32;
    let bottom = top.saturating_add(u32::from(hit.inner_rect.height.saturating_sub(1)));
    let style = if current_only {
        Style::default()
            .fg(panel_contrast_fg(palette))
            .bg(palette.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text).bg(palette.surface1)
    };
    for (index, text_match) in copy_mode.search_matches.iter().enumerate() {
        if (copy_mode.search_current == Some(index)) != current_only
            || text_match.end.row < top
            || text_match.start.row > bottom
        {
            continue;
        }
        let start_row = text_match.start.row.max(top);
        let end_row = text_match.end.row.min(bottom);
        for absolute_row in start_row..=end_row {
            let viewport_row = absolute_row.saturating_sub(top) as u16;
            let start_col = if absolute_row == text_match.start.row {
                text_match.start.col
            } else {
                0
            };
            let end_col = if absolute_row == text_match.end.row {
                text_match.end.col
            } else {
                hit.inner_rect.width.saturating_sub(1)
            };
            for col in start_col..=end_col.min(hit.inner_rect.width.saturating_sub(1)) {
                buffer[(
                    hit.inner_rect.x.saturating_add(col),
                    hit.inner_rect.y.saturating_add(viewport_row),
                )]
                    .set_style(style);
            }
        }
    }
}

fn client_popup_size(size: crate::protocol::ClientShellPopupSize) -> crate::popup_size::PopupSize {
    match size {
        crate::protocol::ClientShellPopupSize::Cells(cells) => {
            crate::popup_size::PopupSize::Cells(cells)
        }
        crate::protocol::ClientShellPopupSize::Percent(percent) => {
            crate::popup_size::PopupSize::Percent(percent)
        }
    }
}
