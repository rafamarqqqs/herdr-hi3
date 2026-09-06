use std::collections::{HashMap, HashSet};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Paragraph, Widget},
};

use super::*;

pub(super) struct AgentRow {
    pub(super) target: AgentRowTarget,
    pub(super) status: crate::api::schema::AgentStatus,
    pub(super) focused: bool,
    pub(super) rows: Vec<Vec<crate::ui::ResolvedToken>>,
    tree: Option<AgentTreeLine>,
}

pub(super) enum AgentRowTarget {
    Pane(String),
    Group(String),
}

struct AgentTreeLine {
    branch: String,
    expanded: Option<bool>,
    label: String,
    trailing: String,
}

pub(super) fn ordered_agent_pane_ids(
    snapshot: &ClientShellSnapshot,
    sort: crate::config::AgentPanelSortConfig,
) -> Vec<String> {
    if snapshot.agent_view_label.is_some() {
        return snapshot
            .agent_order
            .iter()
            .filter(|pane_id| {
                snapshot
                    .agents
                    .iter()
                    .any(|agent| agent.pane_id == pane_id.as_str() && is_real_agent(agent))
            })
            .cloned()
            .collect();
    }
    let mut agents = snapshot
        .agents
        .iter()
        .filter(|agent| is_real_agent(agent))
        .collect::<Vec<_>>();
    if sort == crate::config::AgentPanelSortConfig::Priority {
        agents.sort_by_key(|agent| {
            (
                std::cmp::Reverse(status_priority(agent.agent_status)),
                std::cmp::Reverse(agent.state_change_seq),
            )
        });
    }
    agents
        .into_iter()
        .map(|agent| agent.pane_id.clone())
        .collect()
}

pub(super) fn render_agent_panel(
    buffer: &mut Buffer,
    area: Rect,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    agent_scroll: &mut usize,
    collapsed_groups: &HashSet<String>,
    hits: &mut ShellHitMap,
) {
    if !render_agent_panel_header(
        buffer,
        area,
        snapshot.agent_view_label.as_deref(),
        config,
        hits,
    ) {
        return;
    }

    let rows = if config.agent_panel_sort == crate::config::AgentPanelSortConfig::Tree
        && snapshot.agent_view_label.is_none()
    {
        tree_agent_rows(snapshot, collapsed_groups)
    } else {
        agent_rows(snapshot, config, None)
    };
    render_agent_list(
        buffer,
        area,
        &rows,
        snapshot
            .agent_view_label
            .as_ref()
            .map(|_| " no matching agents"),
        config,
        agent_scroll,
        hits,
        |row| {
            if row.tree.is_some() {
                1
            } else {
                row.rows.len()
            }
        },
        |buffer, rect, row, hits| {
            match &row.target {
                AgentRowTarget::Pane(pane_id) => hits.agents.push((rect, pane_id.clone())),
                AgentRowTarget::Group(key) => hits.agent_groups.push(AgentGroupHit {
                    rect,
                    key: key.clone(),
                    focused: row.focused,
                }),
            }
            render_agent_row(buffer, rect, row, config);
        },
    );
}

pub(super) fn render_agent_panel_header(
    buffer: &mut Buffer,
    area: Rect,
    agent_view_label: Option<&str>,
    config: &ClientShellConfig,
    hits: &mut ShellHitMap,
) -> bool {
    if area.height == 0 {
        return false;
    }
    put_text(
        buffer,
        area.x,
        area.y,
        area.width,
        &"─".repeat(area.width as usize),
        Style::default().fg(config.palette.surface_dim),
    );
    if area.height < 2 {
        return false;
    }
    put_text(
        buffer,
        area.x,
        area.y + 1,
        area.width,
        " agents",
        Style::default()
            .fg(config.palette.overlay0)
            .add_modifier(Modifier::BOLD),
    );
    let sort_label = agent_view_label.unwrap_or(match config.agent_panel_sort {
        crate::config::AgentPanelSortConfig::Spaces => "grouped",
        crate::config::AgentPanelSortConfig::Priority => "priority",
        crate::config::AgentPanelSortConfig::Tree => "tree",
    });
    let sort_label = sort_label.to_owned();
    let sort_width = display_width(&sort_label).min(area.width as usize) as u16;
    let sort_rect = Rect::new(
        area.right().saturating_sub(sort_width),
        area.y + 1,
        sort_width,
        1,
    );
    hits.agent_sort_toggle = if config.mouse_capture && agent_view_label.is_none() {
        sort_rect
    } else {
        Rect::default()
    };
    put_text(
        buffer,
        sort_rect.x,
        sort_rect.y,
        sort_rect.width,
        &sort_label,
        Style::default()
            .fg(if agent_view_label.is_some() {
                config.palette.accent
            } else {
                config.palette.overlay0
            })
            .add_modifier(Modifier::BOLD),
    );
    true
}

pub(super) fn render_agent_list<T>(
    buffer: &mut Buffer,
    area: Rect,
    rows: &[T],
    empty_message: Option<&str>,
    config: &ClientShellConfig,
    agent_scroll: &mut usize,
    hits: &mut ShellHitMap,
    row_lines: impl Fn(&T) -> usize,
    mut render_row: impl FnMut(&mut Buffer, Rect, &T, &mut ShellHitMap),
) {
    let body = Rect::new(
        area.x,
        area.y.saturating_add(3),
        area.width,
        area.height.saturating_sub(3),
    );
    hits.agent_body = body;
    if body.is_empty() || rows.is_empty() {
        *agent_scroll = 0;
        if let Some(message) = empty_message.filter(|_| !body.is_empty()) {
            put_text(
                buffer,
                body.x,
                body.y,
                body.width,
                message,
                Style::default()
                    .fg(config.palette.overlay0)
                    .add_modifier(Modifier::DIM),
            );
        }
        return;
    }

    let row_heights = rows
        .iter()
        .map(|row| row_lines(row).max(1).min(u16::MAX as usize) as u16)
        .collect::<Vec<_>>();
    let gaps = rows
        .iter()
        .enumerate()
        .map(|(index, _)| {
            if index + 1 < rows.len() {
                config.agents.row_gap
            } else {
                0
            }
        })
        .collect::<Vec<_>>();
    let metrics =
        super::scroll::list_scroll_metrics(&row_heights, &gaps, body.height, *agent_scroll);
    hits.agent_max_scroll = metrics.max_offset_from_bottom;
    hits.agent_scroll_metrics = Some(metrics);
    *agent_scroll = metrics
        .max_offset_from_bottom
        .saturating_sub(metrics.offset_from_bottom);
    let show_scrollbar = metrics.max_offset_from_bottom > 0 && body.width > 1;
    let content_width = body.width.saturating_sub(u16::from(show_scrollbar));
    let mut y = body.y;
    for (index, row) in rows.iter().enumerate().skip(*agent_scroll) {
        let height = row_heights[index].min(body.height);
        if y.saturating_add(height) > body.bottom() {
            break;
        }
        let rect = Rect::new(body.x, y, content_width, height);
        render_row(buffer, rect, row, hits);
        y = y
            .saturating_add(height)
            .saturating_add(if index + 1 < rows.len() {
                config.agents.row_gap
            } else {
                0
            });
    }

    if show_scrollbar {
        let track = Rect::new(body.right().saturating_sub(1), body.y, 1, body.height);
        hits.agent_scrollbar = track;
        super::scroll::render_list_scrollbar(buffer, track, metrics, &config.palette);
    }
}

pub(super) fn agent_rows(
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    machine: Option<&str>,
) -> Vec<AgentRow> {
    ordered_agent_pane_ids(snapshot, config.agent_panel_sort)
        .into_iter()
        .filter_map(|pane_id| {
            let agent = snapshot
                .agents
                .iter()
                .find(|agent| agent.pane_id == pane_id)?;
            configured_agent_row(snapshot, config, agent, machine)
        })
        .collect()
}

fn configured_agent_row(
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    agent: &crate::protocol::ClientShellAgent,
    machine: Option<&str>,
) -> Option<AgentRow> {
    let workspace = snapshot
        .workspaces
        .iter()
        .find(|workspace| workspace.workspace_id == agent.workspace_id)?;
    let tab = snapshot.tabs.iter().find(|tab| tab.tab_id == agent.tab_id);
    let pane = snapshot
        .panes
        .iter()
        .find(|pane| pane.pane_id == agent.pane_id);
    let tab_count = snapshot
        .tabs
        .iter()
        .filter(|candidate| candidate.workspace_id == agent.workspace_id)
        .count();
    let tab_label = tab
        .filter(|tab| tab_count > 1 || tab.custom_label)
        .map(|tab| tab.label.as_str());
    let agent_label = agent
        .name
        .as_deref()
        .or(agent.display_agent.as_deref())
        .or(agent.agent.as_deref())
        .or(agent.title.as_deref());
    let labels = agent
        .state_labels
        .iter()
        .cloned()
        .collect::<HashMap<_, _>>();
    let tokens = agent.tokens.iter().cloned().collect::<HashMap<_, _>>();
    let state_text = labels
        .get(status_text(agent.agent_status))
        .map(String::as_str)
        .unwrap_or_else(|| sidebar_status_text(agent.agent_status));
    let canonical_agent = agent
        .agent
        .as_deref()
        .and_then(crate::detect::parse_agent_label);
    let rows = crate::ui::sidebar_agent_rows(
        &config.agents,
        crate::ui::AgentTokenContext {
            machine,
            workspace: &workspace.label,
            tab: tab_label,
            pane: agent
                .title
                .as_deref()
                .or_else(|| pane.and_then(|pane| pane.label.as_deref())),
            agent_label,
            terminal_title: agent.terminal_title.as_deref(),
            terminal_title_stripped: agent.terminal_title_stripped.as_deref(),
            canonical_agent,
            tokens: &tokens,
        },
        state_text,
    );
    Some(AgentRow {
        target: AgentRowTarget::Pane(agent.pane_id.clone()),
        status: agent.agent_status,
        focused: agent.focused,
        rows,
        tree: None,
    })
}

fn tree_agent_rows(
    snapshot: &ClientShellSnapshot,
    collapsed_groups: &HashSet<String>,
) -> Vec<AgentRow> {
    let mut rows = Vec::new();
    for workspace in &snapshot.workspaces {
        let agents = snapshot
            .agents
            .iter()
            .filter(|agent| {
                agent.workspace_id == workspace.workspace_id && is_visible_tree_agent(agent)
            })
            .collect::<Vec<_>>();
        if agents.is_empty() {
            continue;
        }
        let workspace_key = format!("agent-tree:space:{}", workspace.workspace_id);
        let workspace_expanded = !collapsed_groups.contains(&workspace_key);
        let workspace_status = aggregate_status(agents.iter().map(|agent| agent.agent_status));
        rows.push(AgentRow {
            target: AgentRowTarget::Group(workspace_key),
            status: workspace_status,
            focused: workspace.focused,
            rows: Vec::new(),
            tree: Some(AgentTreeLine {
                branch: String::new(),
                expanded: Some(workspace_expanded),
                label: workspace.label.clone(),
                trailing: count_label(agents.len(), "agent", "agents"),
            }),
        });
        if !workspace_expanded {
            continue;
        }

        // The focused agent is the Space's coordinator. Other agents in that
        // Space are rendered as its children, which gives the sidebar a useful
        // agent → subagent shape without exposing tabs or panes. Keep finished
        // agents here while their pane/session is still open so the Space and
        // tree do not disappear unexpectedly. Every agent gets a connector,
        // including a lone coordinator, so the tree remains connected.
        let mut agents = agents;
        agents.sort_by_key(|agent| (!agent.focused, agent.state_change_seq));
        let agent_count = agents.len();
        for (index, agent) in agents.into_iter().enumerate() {
            let label = compact_agent_name(agent);
            let branch = if index + 1 == agent_count {
                "  └─ ".to_owned()
            } else {
                "  ├─ ".to_owned()
            };
            rows.push(AgentRow {
                target: AgentRowTarget::Pane(agent.pane_id.clone()),
                status: agent.agent_status,
                focused: agent.focused,
                rows: Vec::new(),
                tree: Some(AgentTreeLine {
                    branch,
                    expanded: None,
                    label,
                    trailing: sidebar_status_text(agent.agent_status).to_owned(),
                }),
            });
            let subagents = agent
                .tokens
                .iter()
                .filter_map(|(key, value)| key.strip_prefix("__claude_sa:").map(|_| value))
                .collect::<Vec<_>>();
            for (sub_index, kind) in subagents.iter().enumerate() {
                rows.push(AgentRow {
                    target: AgentRowTarget::Pane(agent.pane_id.clone()),
                    status: crate::api::schema::AgentStatus::Working,
                    focused: false,
                    rows: Vec::new(),
                    tree: Some(AgentTreeLine {
                        branch: if sub_index + 1 == subagents.len() {
                            "  └─ ".to_owned()
                        } else {
                            "  ├─ ".to_owned()
                        },
                        expanded: None,
                        label: kind.to_string(),
                        trailing: "subagent".to_owned(),
                    }),
                });
            }
        }
    }
    rows
}

fn aggregate_status(
    statuses: impl Iterator<Item = crate::api::schema::AgentStatus>,
) -> crate::api::schema::AgentStatus {
    statuses
        .max_by_key(|status| status_priority(*status))
        .unwrap_or(crate::api::schema::AgentStatus::Idle)
}

fn count_label(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

fn compact_agent_name(agent: &crate::protocol::ClientShellAgent) -> String {
    agent
        .name
        .as_deref()
        .or(agent.display_agent.as_deref())
        .or(agent.agent.as_deref())
        .or(agent.title.as_deref())
        .unwrap_or("agent")
        .to_owned()
}

fn is_transient_shell(agent: &crate::protocol::ClientShellAgent) -> bool {
    agent
        .tokens
        .iter()
        .any(|(key, value)| key == "__herdr_tree_shell" && value == "1")
}

fn is_real_agent(agent: &crate::protocol::ClientShellAgent) -> bool {
    !is_transient_shell(agent) && (agent.agent.is_some() || agent.display_agent.is_some())
}

fn is_visible_tree_agent(agent: &crate::protocol::ClientShellAgent) -> bool {
    // A real agent can briefly inherit the transient-shell marker while its
    // lifecycle hook is being refreshed. Keep it in the Space tree whenever
    // Herdr still identifies an agent, while hiding plain directory shells.
    !is_transient_shell(agent) || agent.agent.is_some() || agent.display_agent.is_some()
}

pub(super) fn render_agent_row(
    buffer: &mut Buffer,
    rect: Rect,
    row: &AgentRow,
    config: &ClientShellConfig,
) {
    let palette = &config.palette;
    let row_style = if row.focused {
        Style::default().bg(palette.active_row_bg)
    } else {
        Style::default()
    };
    let name_style = if row.focused {
        Style::default()
            .fg(palette.text)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(palette.subtext0)
            .add_modifier(Modifier::BOLD)
    };
    let status_style = Style::default()
        .fg(status_color(row.status, palette))
        .add_modifier(if row.focused {
            Modifier::empty()
        } else {
            Modifier::DIM
        });
    let secondary = Style::default()
        .fg(palette.overlay0)
        .add_modifier(Modifier::DIM);
    let icon = (
        status_icon(row.status, config.status_indicators),
        Style::default().fg(status_color(row.status, palette)),
    );
    buffer.set_style(rect, row_style);
    if row.focused && rect.width > 0 {
        put_text(
            buffer,
            rect.x,
            rect.y,
            1,
            "▌",
            Style::default()
                .fg(palette.accent)
                .bg(palette.active_row_bg),
        );
    }
    if let Some(tree) = &row.tree {
        let content = Rect::new(
            rect.x.saturating_add(1),
            rect.y,
            rect.width.saturating_sub(1),
            1,
        );
        let trailing_width = display_width(&tree.trailing).min(content.width as usize) as u16;
        let left_width = content
            .width
            .saturating_sub(trailing_width)
            .saturating_sub(u16::from(trailing_width > 0));
        let mut spans = vec![ratatui::text::Span::styled(tree.branch.as_str(), secondary)];
        if let Some(expanded) = tree.expanded {
            spans.push(ratatui::text::Span::styled(
                if expanded { "▾ " } else { "▸ " },
                Style::default().fg(palette.accent),
            ));
        }
        spans.extend([
            ratatui::text::Span::styled(icon.0, icon.1),
            ratatui::text::Span::raw(" "),
            ratatui::text::Span::styled(tree.label.as_str(), name_style),
        ]);
        Paragraph::new(Line::from(spans))
            .style(row_style)
            .render(Rect::new(content.x, content.y, left_width, 1), buffer);
        if trailing_width > 0 {
            put_text(
                buffer,
                content.right().saturating_sub(trailing_width),
                content.y,
                trailing_width,
                &tree.trailing,
                secondary,
            );
        }
        return;
    }
    let rows = if row.rows.is_empty() {
        vec![vec![crate::ui::ResolvedToken {
            kind: crate::ui::ResolvedTokenKind::StateIcon,
            style: Default::default(),
        }]]
    } else {
        row.rows.clone()
    };
    for (index, tokens) in rows.iter().take(rect.height as usize).enumerate() {
        let indent = if index == 0 { 1 } else { 3 };
        let mut spans = vec![ratatui::text::Span::raw(" ".repeat(indent))];
        spans.extend(crate::ui::resolved_token_spans(
            tokens,
            icon,
            status_style,
            name_style,
            secondary,
            secondary,
            palette,
            rect.width.saturating_sub(indent as u16) as usize,
        ));
        Paragraph::new(Line::from(spans)).style(row_style).render(
            Rect::new(rect.x, rect.y + index as u16, rect.width, 1),
            buffer,
        );
    }
}

fn put_text(buffer: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    for (offset, character) in text.chars().take(width as usize).enumerate() {
        if let Some(cell) = buffer.cell_mut((x + offset as u16, y)) {
            cell.set_char(character).set_style(style);
        }
    }
}

fn display_width(text: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(text)
}

fn sidebar_status_text(status: crate::api::schema::AgentStatus) -> &'static str {
    use crate::api::schema::AgentStatus;
    match status {
        AgentStatus::Blocked => "blocked",
        AgentStatus::Done => "done",
        AgentStatus::Working => "working",
        AgentStatus::Idle | AgentStatus::Unknown => "idle",
    }
}
