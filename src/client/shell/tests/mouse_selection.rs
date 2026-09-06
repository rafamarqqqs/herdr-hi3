use super::*;

#[test]
fn app_sidebar_toggle_has_padding_and_no_status_overlap() {
    for collapsed in [false, true] {
        for bottom_right in [false, true] {
            let mut config = ClientShellConfig::from_config(&Config::default());
            config.top_padding = 1;
            config.preferences_path = None;
            let mut state = ClientShellState::new(config);
            state.set_snapshot(Box::new(snapshot()));
            state.set_pane_surface(surface());
            state.sidebar_collapsed = collapsed;
            state.compose(106, 30).expect("frame");
            let button = state.hits.sidebar_toggle;
            assert!(button.width >= 4);
            assert_eq!(button.height, 2);
            assert!(!state
                .hits
                .workspaces
                .iter()
                .any(|hit| hit.rect.intersects(button)));
            assert!(!state
                .hits
                .agents
                .iter()
                .any(|(rect, _)| rect.intersects(button)));
            if collapsed {
                assert_eq!(button.width, 5);
                assert_eq!(button.y, 1);
            }
            let (column, row) = if bottom_right {
                (button.right() - 1, button.bottom() - 1)
            } else {
                (button.x, button.y)
            };
            let hover = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Moved,
                column,
                row,
                modifiers: KeyModifiers::NONE,
            })]);
            assert_eq!(hover.mouse_shape, Some(ClientHostMouseShape::Pointer));
            let click = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column,
                row,
                modifiers: KeyModifiers::NONE,
            })]);
            assert_eq!(state.sidebar_collapsed, !collapsed);
            assert!(click.resize);
            assert!(state.chrome_drag.is_none());
        }
    }
}

#[test]
fn app_activity_buttons_have_equal_width_gaps_and_outer_margins() {
    for width in [24, 30, 36] {
        let mut config = ClientShellConfig::from_config(&Config::default());
        config.top_padding = 1;
        let mut state = ClientShellState::new(config);
        state.sidebar_width = width;
        state.set_snapshot(Box::new(snapshot()));
        state.set_pane_surface(surface());
        state.compose(106, 30).expect("frame");
        let [menu, new, explorer] = [
            state.hits.global_launcher,
            state.hits.new_workspace,
            state.hits.explorer_button,
        ];
        assert_eq!(menu.width, new.width);
        assert_eq!(new.width, explorer.width);
        assert_eq!(new.x - menu.right(), explorer.x - new.right());
        let body = state.hits.workspace_body;
        let left = menu.x - body.x;
        let right = body.right() - explorer.right();
        assert!(left >= 1 && right >= 1);
        assert!(left.abs_diff(right) <= 1);
    }
}

#[test]
fn sleep_settings_apply_persist_and_reload_without_server_actions() {
    let path =
        std::env::temp_dir().join(format!("herdr-sleep-settings-{}.json", std::process::id()));
    let mut config = ClientShellConfig::from_config(&Config::default());
    config.top_padding = 1;
    config.preferences_path = Some(path.clone());
    let mut state = ClientShellState::new(config);
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.open_settings_overlay();
    let mut outcome = ClientShellInput::default();
    state.select_settings_section(ClientSettingsSection::Sleep, &mut outcome);
    state.compose(106, 40).expect("settings frame");
    assert!(state
        .hits
        .settings_tabs
        .iter()
        .any(|(rect, section)| *section == ClientSettingsSection::Sleep && !rect.is_empty()));
    assert_eq!(state.hits.settings_choices.len(), 4);
    let Some(ClientShellOverlay::Settings(settings)) = state.overlay.as_mut() else {
        panic!("settings");
    };
    assert_eq!(settings.selected, 1);
    let choice = state.hits.settings_choices[3].0;
    let outcome = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: choice.x,
        row: choice.y,
        modifiers: KeyModifiers::NONE,
    })]);
    assert_eq!(state.sleep_seconds, 30);
    let frame = state.compose(106, 40).expect("updated settings");
    let buffer = frame.to_ratatui_buffer().expect("buffer");
    let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("30 seconds ✓ current"));
    assert!(!text.contains("10 seconds ✓ current"));
    assert!(outcome.actions.is_empty());
    assert!(outcome.requests.is_empty());
    let mut config = ClientShellConfig::from_config(&Config::default());
    config.top_padding = 1;
    config.preferences = super::super::preferences::load(&path).expect("saved preferences");
    let mut restored = ClientShellState::new(config);
    assert_eq!(restored.sleep_seconds, 30);
    restored.set_snapshot(Box::new(snapshot()));
    let now = std::time::Instant::now();
    restored.begin_sleep(now);
    assert_eq!(restored.sleep_remaining, 30);
    assert!(
        !restored
            .tick_sleep(now + std::time::Duration::from_secs(29))
            .detach
    );
    assert_eq!(restored.sleep_remaining, 1);
    assert!(
        restored
            .tick_sleep(now + std::time::Duration::from_secs(30))
            .detach
    );
    std::fs::remove_file(path).expect("remove test preferences");
}

#[test]
fn sleep_is_app_only_and_detaches_without_closing_server_work() {
    let now = std::time::Instant::now();
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.config.top_padding = 0;
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.compose(100, 32).expect("terminal frame");
    assert!(state.hits.sleep_button.is_empty());
    state.begin_sleep(now);
    assert!(state.sleep_started.is_none());
    state.config.top_padding = 1;
    state.sleep_server = "test-host (local server)".into();
    for collapsed in [false, true] {
        state.sidebar_collapsed = collapsed;
        state.compose(100, 32).expect("app frame");
        let button = state.hits.sleep_button;
        assert!(!button.is_empty());
        assert_eq!(button.y, 31);
        assert!(!state
            .hits
            .agents
            .iter()
            .any(|(rect, _)| rect.intersects(button)));
        assert!(!state
            .hits
            .workspaces
            .iter()
            .any(|hit| hit.rect.intersects(button)));
    }
    let button = state.hits.sleep_button;
    let click = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: button.x,
        row: button.y,
        modifiers: KeyModifiers::NONE,
    })]);
    assert!(state.sleep_started.is_some());
    assert!(click.actions.is_empty());
    let started = state.sleep_started.expect("sleep started");
    let phrase = state.sleep_phrase;
    assert_eq!(super::super::sleep::SLEEP_PHRASES.len(), 30);
    assert!(super::super::sleep::SLEEP_PHRASES.contains(&phrase));
    let initial = state
        .compose(100, 32)
        .expect("initial countdown")
        .to_ratatui_buffer()
        .expect("buffer");
    let text: String = initial.content.iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("Closing in 10s"));
    assert!(
        !state
            .tick_sleep(started + std::time::Duration::from_millis(9_999))
            .detach
    );
    let frame = state.compose(100, 32).expect("sleep dialog");
    let buffer = frame.to_ratatui_buffer().expect("buffer");
    let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("Continuing work on"));
    assert!(text.contains("Closing in 1s"));
    assert_eq!(text.matches("esc to cancel").count(), 1);
    assert!(!text.contains("esc  stay here"));
    let rows = buffer
        .content
        .chunks(buffer.area.width as usize)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>();
    let header = rows
        .iter()
        .find(|row| row.contains("Continuing work on"))
        .expect("header");
    assert!(header.contains("Closing in 1s · esc to cancel"));
    assert!(header.find("Continuing work on") < header.find("Closing in"));
    assert_eq!(
        buffer
            .content
            .iter()
            .filter(|cell| ["⠏", "⠹", "⠼", "⠧"].contains(&cell.symbol()))
            .count(),
        1
    );
    assert!(text.contains("test-host (local server)"));
    assert_eq!(state.sleep_phrase, phrase);
    assert!(text.contains(phrase));
    let server_row = rows
        .iter()
        .find(|row| row.contains("test-host (local server)"))
        .expect("server row");
    assert_eq!(
        server_row
            .split("test-host")
            .next()
            .expect("prefix")
            .chars()
            .count(),
        header
            .split("Continuing work on")
            .next()
            .expect("prefix")
            .chars()
            .count()
    );
    let phrase_cells = buffer
        .content
        .windows(phrase.len())
        .find(|cells| cells.iter().map(|cell| cell.symbol()).collect::<String>() == phrase)
        .expect("phrase cells");
    assert!(phrase_cells
        .iter()
        .all(|cell| cell.modifier.contains(Modifier::ITALIC)));
    let input = state.handle_input_bytes(b"do not send this\r");
    assert!(input.requests.is_empty());
    assert!(input.actions.is_empty());
    let done = state.tick_sleep(started + std::time::Duration::from_secs(10));
    assert!(done.detach);
    assert!(done.actions.is_empty());
    assert!(done.requests.is_empty());
}

#[test]
fn sleep_can_be_cancelled_and_survives_tiny_viewports() {
    let now = std::time::Instant::now();
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.config.top_padding = 1;
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.begin_sleep(now);
    for (width, height) in [(100, 32), (35, 10), (5, 3)] {
        let _ = state.compose(width, height);
    }
    state.handle_input_bytes(b"\x1b");
    assert!(state.sleep_started.is_none());
    assert!(
        !state
            .tick_sleep(now + std::time::Duration::from_secs(10))
            .detach
    );
}

#[test]
fn losing_host_focus_cancels_drag_without_switching_tabs() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.chrome_drag = Some(ClientChromeDrag::Pane {
        pane_id: "pane_1".into(),
        origin_tab_id: "tab_1".into(),
        label: None,
        tab_hover: Some(ClientDragTabHover {
            tab_id: "tab_2".into(),
            since: std::time::Instant::now(),
            requested: false,
        }),
        pane_drop_target: None,
        new_tab_drop_target: true,
    });
    state.agent_press = Some(ClientAgentPress {
        label: None,
        pane_id: "pane_1".into(),
        source_tab_id: "tab_1".into(),
        start_column: 1,
        start_row: 1,
        detach_on_click: false,
    });
    let lost = state.handle_raw_events(vec![RawInputEvent::OuterFocusLost]);
    assert!(lost.actions.is_empty());
    assert!(state.chrome_drag.is_none());
    assert!(state.agent_press.is_none());
    assert!(state
        .tick_chrome_drag(std::time::Instant::now() + std::time::Duration::from_secs(1))
        .actions
        .is_empty());
}

#[test]
fn snapshot_agent_exit_and_server_restart_never_resurrect_agents() {
    for restart in [false, true] {
        let mut initial = snapshot();
        initial.agents.push(ClientShellAgent {
            pane_id: "pane_1".into(),
            workspace_id: "ws_1".into(),
            tab_id: "tab_1".into(),
            name: Some("worker".into()),
            agent: Some("codex".into()),
            display_agent: None,
            title: None,
            terminal_title: None,
            terminal_title_stripped: None,
            agent_status: AgentStatus::Working,
            state_change_seq: 1,
            state_labels: Vec::new(),
            tokens: Vec::new(),
            focused: true,
        });
        let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
        state.set_snapshot(Box::new(initial.clone()));
        initial.agents.clear();
        initial.revision += 1;
        if restart {
            initial.boot_id = "new-server".into();
        }
        state.set_snapshot(Box::new(initial));
        assert!(state.snapshot.as_ref().expect("snapshot").agents.is_empty());
    }
}

#[test]
fn app_menu_settings_click_opens_visible_settings() {
    let mut config = ClientShellConfig::from_config(&Config::default());
    config.top_padding = 1;
    let mut state = ClientShellState::new(config);
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.compose(106, 40).expect("app frame");
    let launcher = state.hits.global_launcher;
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: launcher.x + 1,
        row: launcher.y,
        modifiers: KeyModifiers::empty(),
    })]);
    state.compose(106, 40).expect("menu frame");
    let settings = state.hits.global_menu_rows[0].0;
    assert!(settings.y >= launcher.bottom());
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: settings.x,
        row: settings.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(matches!(
        state.overlay,
        Some(ClientShellOverlay::Settings(_))
    ));
    state.compose(106, 40).expect("visible settings");
    assert!(!state.hits.settings_tabs.is_empty());
    assert!(!state.hits.settings_choices.is_empty());
}

#[test]
fn app_unnamed_pane_handle_repositions_and_ignores_stale_drop_targets() {
    for (release_on_target, stacked) in [(true, false), (false, false), (true, true)] {
        let mut config = ClientShellConfig::from_config(&Config::default());
        config.top_padding = 1;
        let mut projected = snapshot();
        let mut second = projected.panes[0].clone();
        second.pane_id = "pane_2".into();
        second.focused = false;
        projected.panes.push(second);
        let mut pane_surface = surface();
        pane_surface.frame = FrameData::from_ratatui_buffer_with_hyperlinks(
            &Buffer::empty(Rect::new(0, 0, 40, 8)),
            None,
            &[],
        );
        pane_surface.panes[0].rect = SurfaceRect {
            x: 0,
            y: 0,
            width: 20,
            height: 8,
        };
        pane_surface.panes[0].inner_rect = SurfaceRect {
            x: 1,
            y: 1,
            width: 18,
            height: 6,
        };
        let mut second = pane_surface.panes[0].clone();
        second.pane_id = "pane_2".into();
        second.rect.x = 20;
        second.inner_rect.x = 21;
        second.focused = false;
        pane_surface.panes.push(second);
        if stacked {
            pane_surface.frame = FrameData::from_ratatui_buffer_with_hyperlinks(
                &Buffer::empty(Rect::new(0, 0, 40, 16)),
                None,
                &[],
            );
            for (index, pane) in pane_surface.panes.iter_mut().enumerate() {
                pane.rect = SurfaceRect {
                    x: 0,
                    y: index as u16 * 8,
                    width: 40,
                    height: 8,
                };
                pane.inner_rect = SurfaceRect {
                    x: 1,
                    y: index as u16 * 8 + 1,
                    width: 38,
                    height: 6,
                };
            }
        }
        let mut state = ClientShellState::new(config);
        state.set_snapshot(Box::new(projected));
        state.set_pane_surface(pane_surface);
        let frame = state.compose(106, 40).expect("two unnamed panes");
        let first = state.hits.panes[0].rect;
        let second = state.hits.panes[1].rect;
        let drop_point = if stacked {
            (second.x, second.y + second.height / 2)
        } else {
            (second.x + second.width / 2, second.y)
        };
        let hover =
            state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
                kind: MouseEventKind::Moved,
                column: second.x + 2,
                row: second.y,
                modifiers: KeyModifiers::empty(),
            })]);
        assert_eq!(hover.mouse_shape, Some(ClientHostMouseShape::Pointer));
        let hovered = state
            .compose(106, 40)
            .expect("hovered unnamed grip")
            .to_ratatui_buffer()
            .expect("buffer");
        assert_ne!(
            hovered[(second.x + 2, second.y)].bg,
            hovered[(second.x + 5, second.y)].bg
        );
        assert_ne!(
            hovered[(second.x + 2, second.y)].bg,
            state.config.palette.active_row_bg
        );
        assert_eq!(
            frame.to_ratatui_buffer().expect("buffer")[(first.x + 2, first.y)].symbol(),
            "⠿"
        );
        for (kind, column, row) in [
            (
                MouseEventKind::Down(MouseButton::Left),
                first.x + 2,
                first.y,
            ),
            (
                MouseEventKind::Drag(MouseButton::Left),
                drop_point.0,
                drop_point.1,
            ),
        ] {
            state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
                kind,
                column,
                row,
                modifiers: KeyModifiers::empty(),
            })]);
        }
        let Some(ClientChromeDrag::Pane {
            pane_drop_target: Some(target),
            ..
        }) = state.chrome_drag.as_ref()
        else {
            panic!("preview target");
        };
        let preview_rect = target.highlight_rect();
        assert_eq!(target.rect, first.union(second));
        let preview = state
            .compose(106, 40)
            .expect("swap preview")
            .to_ratatui_buffer()
            .expect("buffer");
        assert_ne!(
            preview[(preview_rect.x, preview_rect.y)].bg,
            state.config.palette.accent
        );
        assert_eq!(
            preview[(preview_rect.x, preview_rect.y)].fg,
            frame.to_ratatui_buffer().expect("original")[(preview_rect.x, preview_rect.y)].fg
        );
        let (column, row) = if release_on_target {
            drop_point
        } else {
            (105, 39)
        };
        let result =
            state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column,
                row,
                modifiers: KeyModifiers::empty(),
            })]);
        if release_on_target {
            assert!(
                matches!(&result.actions[..], [ClientShellAction::Endpoint { request, .. }]
                if matches!(&request.method, crate::api::schema::Method::PaneReposition(params)
                    if params.pane_id == "pane_1"
                        && params.target_pane_id == "pane_2"
                        && params.direction == if stacked { crate::api::schema::TabMergeDirection::Left } else { crate::api::schema::TabMergeDirection::Up }))
            );
        } else {
            assert!(result.actions.is_empty());
        }
    }
}

#[test]
fn ctrl_click_routes_link_activation_through_endpoint_then_client_host() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("pane frame");
    let pane = state.hits.panes[0].clone();
    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: pane.inner_rect.x + 2,
        row: pane.inner_rect.y + 1,
        modifiers: KeyModifiers::CONTROL,
    };
    let activate = state.handle_raw_events(vec![RawInputEvent::Mouse(down)]);
    let [ClientShellAction::Endpoint { request, .. }] = &activate.actions[..] else {
        panic!("expected link activation request");
    };
    let request_id = request.id.clone();
    assert!(matches!(
        &request.method,
        crate::api::schema::Method::PaneLinkActivate(params)
            if params.pane_id == "pane_1" && params.viewport_row == 1 && params.col == 2
    ));

    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        ..down
    };
    let held = state.handle_raw_events(vec![RawInputEvent::Mouse(up)]);
    assert!(held.requests.is_empty() && held.actions.is_empty());
    let (_, actions) = state.handle_endpoint_result(
        "boot-1",
        &request_id,
        Ok(crate::api::schema::ResponseResult::PaneLinkActivated {
            url: Some("https://example.test".to_owned()),
            handled: false,
        }),
    );
    assert!(matches!(
        &actions[..],
        [ClientShellAction::OpenSafeWebUrl(url)] if url == "https://example.test"
    ));
    assert!(!state.url_click_consumes_until_up);
}

#[test]
fn ctrl_click_without_a_link_replays_the_original_gesture() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("pane frame");
    let pane = state.hits.panes[0].clone();
    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: pane.inner_rect.x + 2,
        row: pane.inner_rect.y + 1,
        modifiers: KeyModifiers::CONTROL,
    };
    let activate = state.handle_raw_events(vec![RawInputEvent::Mouse(down)]);
    let request_id = match &activate.actions[..] {
        [ClientShellAction::Endpoint { request, .. }] => request.id.clone(),
        _ => panic!("expected link activation request"),
    };
    let (_, actions) = state.handle_endpoint_result(
        "boot-1",
        &request_id,
        Ok(crate::api::schema::ResponseResult::PaneLinkActivated {
            url: None,
            handled: false,
        }),
    );
    assert!(matches!(
        &actions[..],
        [ClientShellAction::ReplayMouse(events)] if events == &vec![down]
    ));
    let replay = match actions.into_iter().next().expect("replay action") {
        ClientShellAction::ReplayMouse(events) => state.replay_mouse_events(events),
        _ => unreachable!(),
    };
    assert!(matches!(
        &replay.actions[..],
        [ClientShellAction::Endpoint { request, .. }]
            if matches!(request.method, crate::api::schema::Method::PaneFocus(_))
    ));
    assert!(state.selection.is_some());
}

#[test]
fn pane_split_drag_uses_projected_handle_and_stable_tab_path() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    let mut pane_surface = surface();
    pane_surface.splits.push(PaneSurfaceSplit {
        direction: PaneSurfaceSplitDirection::Horizontal,
        pos: 40,
        area: SurfaceRect {
            x: 0,
            y: 0,
            width: 80,
            height: 19,
        },
        hit_rect: SurfaceRect {
            x: 40,
            y: 0,
            width: 1,
            height: 19,
        },
        path: vec![false, true],
    });
    state.set_pane_surface(pane_surface);
    state.compose(106, 20).expect("split pane surface");
    let split = state.hits.pane_splits[0].clone();

    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: split.hit_rect.x,
        row: split.hit_rect.y + 2,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(matches!(
        state.chrome_drag,
        Some(ClientChromeDrag::PaneSplit { .. })
    ));
    let mut replacement = snapshot();
    replacement.revision = 2;
    replacement
        .tab_bar_right
        .push(crate::protocol::ClientShellTabStatusSegment {
            text: "updated".into(),
            accent: false,
        });
    let mut replacement_surface = surface();
    replacement_surface.projection_revision = 2;
    replacement_surface.splits.push(PaneSurfaceSplit {
        direction: PaneSurfaceSplitDirection::Horizontal,
        pos: 40,
        area: SurfaceRect {
            x: 0,
            y: 0,
            width: 80,
            height: 19,
        },
        hit_rect: SurfaceRect {
            x: 40,
            y: 0,
            width: 1,
            height: 19,
        },
        path: vec![false, true],
    });
    state.set_snapshot(Box::new(replacement));
    state.set_pane_surface(replacement_surface);
    let drag = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: split.area.x + 48,
        row: split.hit_rect.y + 2,
        modifiers: KeyModifiers::empty(),
    })]);
    let [ClientShellAction::Endpoint { request, .. }] = &drag.actions[..] else {
        panic!("pane split drag should use endpoint API");
    };
    assert!(matches!(
        &request.method,
        crate::api::schema::Method::LayoutSetSplitRatio(params)
            if params.tab_id.as_deref() == Some("tab_1")
                && params.path == vec![false, true]
                && (params.ratio - 0.6).abs() < f32::EPSILON
    ));
    let release =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: split.area.x + 48,
            row: split.hit_rect.y + 2,
            modifiers: KeyModifiers::empty(),
        })]);
    assert!(release.actions.is_empty());
    assert!(state.chrome_drag.is_none());
}

#[test]
fn disabled_mouse_chrome_keeps_tab_wheel_but_removes_split_drag_hits() {
    let mut config = Config::default();
    config.ui.mouse_capture = false;
    let mut projected = snapshot();
    let mut second_tab = projected.tabs[0].clone();
    second_tab.tab_id = "tab_2".into();
    second_tab.number = 2;
    second_tab.label = "2".into();
    second_tab.focused = false;
    projected.tabs.push(second_tab);
    let mut pane_surface = surface();
    pane_surface.splits.push(PaneSurfaceSplit {
        direction: PaneSurfaceSplitDirection::Horizontal,
        pos: 40,
        area: SurfaceRect {
            x: 0,
            y: 0,
            width: 80,
            height: 19,
        },
        hit_rect: SurfaceRect {
            x: 40,
            y: 0,
            width: 1,
            height: 19,
        },
        path: Vec::new(),
    });
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&config));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(pane_surface);
    state.compose(106, 20).expect("mouse-disabled shell");
    assert!(state.hits.pane_splits.is_empty());
    let first_tab = state.hits.tabs[0].0;
    let wheel = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: first_tab.x,
        row: first_tab.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(matches!(
        &wheel.actions[..],
        [ClientShellAction::Endpoint { request, .. }]
            if matches!(
                &request.method,
                crate::api::schema::Method::TabFocus(target) if target.tab_id == "tab_2"
            )
    ));
}

#[test]
fn client_double_click_selects_and_copies_endpoint_row_word() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("composed frame");
    let pane = state.hits.panes[0].clone();
    let click = || {
        RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: pane.inner_rect.x + 1,
            row: pane.inner_rect.y,
            modifiers: KeyModifiers::empty(),
        })
    };
    let release = || {
        RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: pane.inner_rect.x + 1,
            row: pane.inner_rect.y,
            modifiers: KeyModifiers::empty(),
        })
    };

    state.handle_raw_events(vec![click()]);
    state.handle_raw_events(vec![release()]);
    let second = state.handle_raw_events(vec![click()]);
    let ClientShellAction::Endpoint { request, .. } = second
        .actions
        .iter()
        .find(|action| {
            matches!(
                action,
                ClientShellAction::Endpoint { request, .. }
                    if matches!(request.method, crate::api::schema::Method::PaneSelectionRead(_))
            )
        })
        .expect("word-row read")
    else {
        unreachable!()
    };
    let word_request_id = request.id.clone();
    assert!(matches!(
        &request.method,
        crate::api::schema::Method::PaneSelectionRead(params)
            if params.anchor == crate::api::schema::PaneTextPoint { row: 0, col: 0 }
                && params.cursor == crate::api::schema::PaneTextPoint { row: 0, col: 3 }
    ));

    let (repaint, actions) = state.handle_endpoint_result(
        "boot-1",
        &word_request_id,
        Ok(crate::api::schema::ResponseResult::PaneSelection {
            pane_id: "pane_1".into(),
            text: "LIVE".into(),
        }),
    );
    assert!(repaint);
    assert!(state
        .selection
        .as_ref()
        .is_some_and(crate::selection::Selection::is_finalized));
    let [ClientShellAction::Endpoint { request, .. }] = &actions[..] else {
        panic!("auto-copy should read the selected word");
    };
    let copy_request_id = request.id.clone();
    assert!(matches!(
        &request.method,
        crate::api::schema::Method::PaneSelectionRead(params)
            if params.anchor.col == 0 && params.cursor.col == 3
    ));
    let (_, actions) = state.handle_endpoint_result(
        "boot-1",
        &copy_request_id,
        Ok(crate::api::schema::ResponseResult::PaneSelection {
            pane_id: "pane_1".into(),
            text: "LIVE".into(),
        }),
    );
    assert!(matches!(
        &actions[..],
        [ClientShellAction::ClipboardWrite(bytes)] if bytes == b"LIVE"
    ));
}

#[test]
fn pane_content_updates_preserve_active_selection_only_when_selected_cells_stay_stable() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    let surface_at = |surface_revision, content_revision, alternate_screen_active| {
        let mut pane_surface = surface();
        pane_surface.surface_revision = surface_revision;
        pane_surface.panes[0].content_revision = content_revision;
        pane_surface.panes[0].scroll = Some(crate::protocol::PaneSurfaceScrollMetrics {
            offset_from_bottom: 0,
            max_offset_from_bottom: 11,
            viewport_rows: 2,
        });
        pane_surface.panes[0].alternate_screen_active = alternate_screen_active;
        pane_surface
    };
    state.set_pane_surface(surface_at(1, 0, true));
    state.compose(106, 20).expect("composed frame");
    let pane = state.hits.panes[0].clone();
    let mouse = |kind, column, row| {
        RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::empty(),
        })
    };

    state.handle_raw_events(vec![mouse(
        MouseEventKind::Down(MouseButton::Left),
        pane.inner_rect.x,
        pane.inner_rect.y + 1,
    )]);
    let mut updated_surface = surface_at(2, 2, true);
    updated_surface.frame.cells[0].symbol = "W".into();
    state.set_pane_surface(updated_surface);
    state.compose(106, 20).expect("updated frame");

    let drag = state.handle_raw_events(vec![mouse(
        MouseEventKind::Drag(MouseButton::Left),
        pane.inner_rect.x + 1,
        pane.inner_rect.y + 1,
    )]);

    assert!(drag.repaint);
    let selection = state.selection.as_ref().expect("visible selection");
    assert!(selection.is_visible());
    assert_eq!(selection.ordered_cells(), ((12, 0), (12, 1)));

    state.selection = Some(crate::selection::Selection::absolute_anchor(
        "pane_1".to_owned(),
        (12, 0),
    ));
    let mut replaced_surface = surface_at(3, 4, true);
    replaced_surface.frame.cells[4].symbol = "X".into();
    state.set_pane_surface(replaced_surface);
    assert!(state.selection.is_none());

    for (surface_revision, content_revision, width, alternate_screen_active) in
        [(4, 6, 4, false), (5, 8, 3, false), (6, 9, 3, false)]
    {
        state.selection = Some(crate::selection::Selection::absolute_anchor(
            "pane_1".to_owned(),
            (12, 0),
        ));
        let mut changed_surface =
            surface_at(surface_revision, content_revision, alternate_screen_active);
        changed_surface.panes[0].inner_rect.width = width;
        changed_surface.panes[0].alternate_screen_active = alternate_screen_active;
        state.set_pane_surface(changed_surface);
        assert!(state.selection.is_none());
    }
}

#[test]
fn pane_mouse_input_keeps_stable_target_and_endpoint_encoding() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    let mut pane_surface = surface();
    pane_surface.panes[0].mouse_reporting = true;
    state.set_pane_surface(pane_surface);
    state.compose(106, 20).expect("composed frame");
    let pane = state.hits.panes[0].clone();

    let click = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: pane.inner_rect.x + 2,
        row: pane.inner_rect.y + 1,
        modifiers: KeyModifiers::ALT,
    })]);
    let [ClientMessage::ClientShellPaneInput { pane_id, events }] = &click.requests[..] else {
        panic!("pane application click should use targeted canonical input");
    };
    assert_eq!(pane_id, "pane_1");
    assert!(matches!(
        &events[..],
        [ClientPaneInputEvent::Mouse {
            kind: crate::protocol::ClientMouseKind::Down(
                crate::protocol::ClientMouseButton::Left
            ),
            position: ClientMousePosition::Cell { column: 2, row: 1 },
            modifiers,
            ..
        }] if *modifiers == KeyModifiers::ALT.bits()
    ));
    let moved = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Moved,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::ALT,
    })]);
    assert!(moved.requests.is_empty());
    assert!(state.pane_mouse_gesture.is_some());
    state.hits.panes.clear();
    let release =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::ALT,
        })]);
    assert!(matches!(
        &release.requests[..],
        [ClientMessage::ClientShellPaneInput { pane_id, events }]
            if pane_id == "pane_1"
                && matches!(
                    &events[..],
                    [ClientPaneInputEvent::Mouse {
                        kind: crate::protocol::ClientMouseKind::Up(
                            crate::protocol::ClientMouseButton::Left
                        ),
                        ..
                    }]
                )
    ));
    assert!(state.pane_mouse_gesture.is_none());
}

#[test]
fn pane_pixel_mouse_preserves_pane_relative_pixel_coordinates() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    let mut pane_surface = surface();
    pane_surface.panes[0].mouse_reporting = true;
    pane_surface.panes[0].sgr_pixel_mouse = true;
    pane_surface.panes[0].pixel_width = 39;
    pane_surface.panes[0].pixel_height = 38;
    state.set_pane_surface(pane_surface);
    state.compose(106, 20).expect("composed frame");
    let pane = state.hits.panes[0].clone();
    let geometry =
        crate::input::mouse::HostGeometry::new(106, 20, 1060, 400).expect("host geometry");
    let x = u32::from(pane.inner_rect.x) * 10 + 21;
    let y = u32::from(pane.inner_rect.y) * 20 + 21;
    let report = format!("\x1b[<0;{x};{y}M");

    let outcome = state.handle_pixel_mouse(report.as_bytes(), geometry);
    assert!(matches!(
        &outcome.requests[..],
        [ClientMessage::ClientShellPaneInput { pane_id, events }]
            if pane_id == "pane_1"
                && matches!(
                    &events[..],
                    [ClientPaneInputEvent::Mouse {
                        kind: crate::protocol::ClientMouseKind::Down(
                            crate::protocol::ClientMouseButton::Left
                        ),
                        position: ClientMousePosition::Pixels { x: 20, y: 20, .. },
                        ..
                    }]
                )
    ));

    let lost = state.handle_raw_events(vec![RawInputEvent::OuterFocusLost]);
    assert!(matches!(
        &lost.requests[..],
        [
            ClientMessage::ClientShellPaneInput { pane_id, events },
            ClientMessage::ClientShellFocus { focused: false }
        ] if pane_id == "pane_1" && matches!(
            &events[..],
            [ClientPaneInputEvent::Mouse {
                kind: crate::protocol::ClientMouseKind::Up(
                    crate::protocol::ClientMouseButton::Left
                ),
                position: ClientMousePosition::Pixels { x: 20, y: 20, .. },
                ..
            }]
        )
    ));
}

#[test]
fn pane_owned_right_click_forwards_the_complete_gesture() {
    let mut snapshot = snapshot();
    snapshot.panes[0].right_click_passthrough = true;
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot));
    let mut pane_surface = surface();
    pane_surface.panes[0].mouse_reporting = true;
    state.set_pane_surface(pane_surface);
    state.compose(106, 20).expect("composed frame");
    let pane = state.hits.panes[0].clone();

    let down = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Right),
        column: pane.inner_rect.x + 1,
        row: pane.inner_rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(matches!(
        &down.requests[..],
        [ClientMessage::ClientShellPaneInput { pane_id, .. }] if pane_id == "pane_1"
    ));
    assert!(state.overlay.is_none());
    assert!(state.pane_mouse_gesture.is_some());

    let up = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Right),
        column: 0,
        row: 0,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(matches!(
        &up.requests[..],
        [ClientMessage::ClientShellPaneInput { pane_id, events }]
            if pane_id == "pane_1"
                && matches!(
                    &events[..],
                    [ClientPaneInputEvent::Mouse {
                        kind: crate::protocol::ClientMouseKind::Up(
                            crate::protocol::ClientMouseButton::Right
                        ),
                        ..
                    }]
                )
    ));
    assert!(state.pane_mouse_gesture.is_none());
}

#[test]
fn tab_click_waits_for_release_and_drag_reorders_by_stable_id() {
    let mut projected = snapshot();
    for index in 2..=3 {
        let mut tab = projected.tabs[0].clone();
        tab.tab_id = format!("tab_{index}");
        tab.number = index;
        tab.label = index.to_string();
        tab.focused = false;
        projected.tabs.push(tab);
    }
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("three tabs");
    let first = state.hits.tabs[0].0;
    let third = state.hits.tabs[2].0;

    let down = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: first.x + 1,
        row: first.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(down.actions.is_empty());
    let drag = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: third.right().saturating_sub(1),
        row: third.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(drag.repaint);
    assert!(matches!(
        state.chrome_drag,
        Some(ClientChromeDrag::Tab {
            ref tab_id,
            insert_index: Some(3),
            ..
        }) if tab_id == "tab_1"
    ));
    let frame = state.compose(106, 20).expect("tab drop indicator");
    assert!(frame
        .cells
        .iter()
        .take(frame.width as usize)
        .any(|cell| cell.symbol == "│"));

    let release =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: third.right().saturating_sub(1),
            row: third.y,
            modifiers: KeyModifiers::empty(),
        })]);
    let [ClientShellAction::Endpoint { request, .. }] = &release.actions[..] else {
        panic!("tab drag should use endpoint API");
    };
    assert!(matches!(
        &request.method,
        crate::api::schema::Method::TabMove(params)
            if params.tab_id == "tab_1" && params.insert_index == 3
    ));

    state.compose(106, 20).expect("tabs after drag");
    let second = state.hits.tabs[1].0;
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: second.x + 1,
        row: second.y,
        modifiers: KeyModifiers::empty(),
    })]);
    let click = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: second.x + 1,
        row: second.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(matches!(
        &click.actions[0],
        ClientShellAction::Endpoint { request, .. }
            if matches!(&request.method, crate::api::schema::Method::TabFocus(target) if target.tab_id == "tab_2")
    ));
}

#[test]
fn tab_click_jitter_does_not_merge_into_the_visible_pane() {
    for offset in [(0, 0), (1, 0), (0, 1), (2, 0), (0, 2), (3, 0), (0, 3)] {
        let mut projected = snapshot();
        let mut tab = projected.tabs[0].clone();
        tab.tab_id = "tab_2".into();
        tab.number = 2;
        tab.label = "second".into();
        tab.focused = false;
        projected.tabs.push(tab);
        let mut config = ClientShellConfig::from_config(&Config::default());
        config.top_padding = 1;
        let mut state = ClientShellState::new(config);
        state.set_snapshot(Box::new(projected));
        state.set_pane_surface(surface());
        state.compose(106, 20).expect("app tabs");
        let tab = state.hits.tabs[1].0;
        let mut actions = Vec::new();
        for (kind, column, row) in [
            (MouseEventKind::Down(MouseButton::Left), tab.x + 1, tab.y),
            (
                MouseEventKind::Drag(MouseButton::Left),
                tab.x + 1 + offset.0,
                tab.y + offset.1,
            ),
            (
                MouseEventKind::Up(MouseButton::Left),
                tab.x + 1 + offset.0,
                tab.y + offset.1,
            ),
        ] {
            actions.extend(
                state
                    .handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
                        kind,
                        column,
                        row,
                        modifiers: KeyModifiers::empty(),
                    })])
                    .actions,
            );
            // Selection must already be sent on Down, even if Up is lost.
            assert_eq!(actions.len(), 1);
        }
        assert!(
            matches!(&actions[..], [ClientShellAction::Endpoint { request, .. }]
            if matches!(&request.method, crate::api::schema::Method::TabFocus(target)
                if target.tab_id == "tab_2"))
        );
        assert!(state.chrome_drag.is_none());
    }
}

#[test]
fn tab_drop_requires_an_edge_preview_not_just_mouse_release() {
    for center in [true, false] {
        let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
        state.config.top_padding = 1;
        state.set_snapshot(Box::new(snapshot()));
        let mut pane_surface = surface();
        pane_surface.frame = FrameData::from_ratatui_buffer_with_hyperlinks(
            &Buffer::empty(Rect::new(0, 0, 40, 16)),
            None,
            &[],
        );
        pane_surface.panes[0].rect = SurfaceRect {
            x: 0,
            y: 0,
            width: 40,
            height: 16,
        };
        pane_surface.panes[0].inner_rect = pane_surface.panes[0].rect;
        state.set_pane_surface(pane_surface);
        state.compose(106, 20).expect("frame");
        let pane = state.hits.panes[0].rect;
        state.chrome_drag = Some(ClientChromeDrag::Tab {
            tab_id: "tab_2".into(),
            workspace_id: "ws_1".into(),
            insert_index: None,
            origin_tab_id: "tab_1".into(),
            tab_hover: None,
            pane_drop_target: None,
        });
        let column = if center {
            pane.x + pane.width / 2
        } else {
            pane.x
        };
        let row = pane.y + pane.height / 2;
        if center {
            state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Drag(MouseButton::Left),
                column,
                row,
                modifiers: KeyModifiers::NONE,
            })]);
        }
        let result = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        })]);
        assert!(result.actions.is_empty());
    }
}

#[test]
fn tab_drag_released_on_tab_row_does_not_use_stale_pane_target() {
    let mut projected = snapshot();
    let mut tab = projected.tabs[0].clone();
    tab.tab_id = "tab_2".into();
    tab.number = 2;
    tab.label = "second".into();
    tab.focused = false;
    projected.tabs.push(tab);
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("two tabs");
    let tab = state.hits.tabs[1].0;
    let pane = state.hits.panes[0].rect;
    for (kind, column, row) in [
        (MouseEventKind::Down(MouseButton::Left), tab.x + 1, tab.y),
        (
            MouseEventKind::Drag(MouseButton::Left),
            pane.x + 2,
            pane.y + 1,
        ),
    ] {
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::empty(),
        })]);
    }
    assert!(matches!(
        state.chrome_drag,
        Some(ClientChromeDrag::Tab {
            pane_drop_target: Some(_),
            ..
        })
    ));
    let release =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: tab.x + 1,
            row: tab.y,
            modifiers: KeyModifiers::empty(),
        })]);
    assert!(
        matches!(&release.actions[..], [ClientShellAction::Endpoint { request, .. }]
        if matches!(&request.method, crate::api::schema::Method::TabMove(_)))
    );
}

#[test]
fn tab_drag_clears_its_drop_target_after_leaving_the_tab_row() {
    let mut projected = snapshot();
    for index in 2..=3 {
        let mut tab = projected.tabs[0].clone();
        tab.tab_id = format!("tab_{index}");
        tab.number = index;
        tab.label = index.to_string();
        tab.focused = false;
        projected.tabs.push(tab);
    }
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("three tabs");
    let first = state.hits.tabs[0].0;
    let third = state.hits.tabs[2].0;
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: first.x + 1,
        row: first.y,
        modifiers: KeyModifiers::empty(),
    })]);
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: third.x,
        row: third.y,
        modifiers: KeyModifiers::empty(),
    })]);
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: third.x,
        row: third.y + 1,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(matches!(
        state.chrome_drag,
        Some(ClientChromeDrag::Tab {
            insert_index: None,
            ..
        })
    ));
    let release =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: third.x,
            row: third.y + 1,
            modifiers: KeyModifiers::empty(),
        })]);
    assert!(release.actions.is_empty());
}

#[test]
fn tab_drag_hovers_target_tab_then_merges_into_a_pane_edge() {
    let mut projected = snapshot();
    projected.tabs[0].label = "X".into();
    projected.tabs[0].custom_label = true;
    let mut second_tab = projected.tabs[0].clone();
    second_tab.tab_id = "tab_2".into();
    second_tab.number = 2;
    second_tab.label = "2".into();
    second_tab.focused = false;
    projected.tabs.push(second_tab);
    let mut second_pane = projected.panes[0].clone();
    second_pane.pane_id = "pane_2".into();
    second_pane.tab_id = "tab_2".into();
    second_pane.focused = false;
    projected.panes.push(second_pane);
    let mut config = ClientShellConfig::from_config(&Config::default());
    config.top_padding = 1;
    let mut state = ClientShellState::new(config);
    state.set_snapshot(Box::new(projected.clone()));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("two tabs");
    let source_tab = state.hits.tabs[0].0;
    let target_tab = state.hits.tabs[1].0;

    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: source_tab.x + 1,
        row: source_tab.y,
        modifiers: KeyModifiers::empty(),
    })]);
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: target_tab.x + 1,
        row: target_tab.y,
        modifiers: KeyModifiers::empty(),
    })]);
    let hover =
        state.tick_chrome_drag(std::time::Instant::now() + std::time::Duration::from_millis(500));
    assert!(matches!(
        &hover.actions[..],
        [ClientShellAction::Endpoint { request, .. }]
            if matches!(&request.method, crate::api::schema::Method::TabFocus(target) if target.tab_id == "tab_2")
    ));

    projected.revision = 2;
    projected.focused_tab_id = Some("tab_2".into());
    projected.focused_pane_id = Some("pane_2".into());
    projected.workspaces[0].active_tab_id = "tab_2".into();
    for tab in &mut projected.tabs {
        tab.focused = tab.tab_id == "tab_2";
    }
    for pane in &mut projected.panes {
        pane.focused = pane.pane_id == "pane_2";
    }
    state.set_snapshot(Box::new(projected));
    let mut target_surface = surface();
    target_surface.projection_revision = 2;
    target_surface.panes[0].pane_id = "pane_2".into();
    state.set_pane_surface(target_surface);
    state.compose(106, 20).expect("target pane");
    let target_pane = state.hits.panes[0].clone();
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: target_pane.rect.x,
        row: target_pane.rect.y + target_pane.rect.height / 2,
        modifiers: KeyModifiers::empty(),
    })]);
    let release =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: target_pane.rect.x,
            row: target_pane.rect.y + target_pane.rect.height / 2,
            modifiers: KeyModifiers::empty(),
        })]);

    assert!(matches!(
        &release.actions[..],
        [ClientShellAction::Endpoint { request: rename, .. }, ClientShellAction::Endpoint { request, .. }]
            if matches!(
                &request.method,
                crate::api::schema::Method::TabMerge(params)
                    if params.source_tab_id == "tab_1"
                        && params.target_tab_id == "tab_2"
                        && params.target_pane_id == "pane_2"
                        && params.direction == crate::api::schema::TabMergeDirection::Left
            ) && matches!(&rename.method, crate::api::schema::Method::PaneRename(params)
                if params.pane_id == "pane_1" && params.label.as_deref() == Some("X"))
    ));
}

#[test]
fn sidebar_agent_drag_does_not_move_panes() {
    let mut projected = snapshot();
    let mut second_pane = projected.panes[0].clone();
    second_pane.pane_id = "pane_2".into();
    second_pane.focused = false;
    projected.panes.push(second_pane);
    projected.agent_order = vec!["pane_1".into()];
    projected.agents.push(ClientShellAgent {
        pane_id: "pane_1".into(),
        workspace_id: "ws_1".into(),
        tab_id: "tab_1".into(),
        name: Some("worker".into()),
        display_agent: Some("codex".into()),
        agent: Some("codex".into()),
        title: None,
        terminal_title: None,
        terminal_title_stripped: None,
        agent_status: AgentStatus::Idle,
        state_change_seq: 1,
        state_labels: Vec::new(),
        tokens: Vec::new(),
        focused: true,
    });
    let mut pane_surface = surface();
    pane_surface.frame = FrameData::from_ratatui_buffer_with_hyperlinks(
        &Buffer::with_lines([
            "LEFT    RIGHT   ",
            "PANE    PANE    ",
            "                ",
            "                ",
            "                ",
            "                ",
            "                ",
            "                ",
        ]),
        None,
        &[],
    );
    pane_surface.panes[0].rect.width = 8;
    pane_surface.panes[0].rect.height = 8;
    pane_surface.panes[0].inner_rect.width = 8;
    pane_surface.panes[0].inner_rect.height = 8;
    let mut second_surface_pane = pane_surface.panes[0].clone();
    second_surface_pane.pane_id = "pane_2".into();
    second_surface_pane.rect.x = 8;
    second_surface_pane.inner_rect.x = 8;
    second_surface_pane.focused = false;
    pane_surface.panes.push(second_surface_pane);
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(pane_surface);
    state.compose(106, 20).expect("agent and two panes");
    let agent = state.hits.agents[0].0;
    let target = state
        .hits
        .panes
        .iter()
        .find(|pane| pane.pane_id == "pane_2")
        .cloned()
        .expect("second pane hit");

    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: agent.x + 1,
        row: agent.y,
        modifiers: KeyModifiers::empty(),
    })]);
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: target.rect.x + target.rect.width / 2,
        row: target.rect.y + target.rect.height / 2,
        modifiers: KeyModifiers::empty(),
    })]);
    let release =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: target.rect.x + target.rect.width / 2,
            row: target.rect.y + target.rect.height / 2,
            modifiers: KeyModifiers::empty(),
        })]);

    assert!(release.actions.is_empty());
}

#[test]
fn sidebar_agent_drag_onto_plus_does_not_detach() {
    let mut projected = snapshot();
    projected.agent_order = vec!["pane_1".into()];
    projected.agents.push(ClientShellAgent {
        pane_id: "pane_1".into(),
        workspace_id: "ws_1".into(),
        tab_id: "tab_1".into(),
        name: Some("worker".into()),
        display_agent: Some("codex".into()),
        agent: Some("codex".into()),
        title: None,
        terminal_title: None,
        terminal_title_stripped: None,
        agent_status: AgentStatus::Idle,
        state_change_seq: 1,
        state_labels: Vec::new(),
        tokens: Vec::new(),
        focused: true,
    });
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("agent and new-tab button");
    let agent = state.hits.agents[0].0;
    let new_tab = state.hits.new_tab;
    assert!(!new_tab.is_empty());

    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: agent.x + 1,
        row: agent.y,
        modifiers: KeyModifiers::empty(),
    })]);
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: new_tab.x + new_tab.width / 2,
        row: new_tab.y,
        modifiers: KeyModifiers::empty(),
    })]);
    let release =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: new_tab.x + new_tab.width / 2,
            row: new_tab.y,
            modifiers: KeyModifiers::empty(),
        })]);

    assert!(release.actions.is_empty());
}

#[test]
fn pane_label_drag_onto_new_tab_button_restores_named_tab() {
    let mut projected = snapshot();
    projected.panes[0].label = Some("reviewer".into());
    let mut pane_surface = surface();
    let mut legacy_header = Buffer::empty(Rect::new(0, 0, 20, 8));
    legacy_header.set_string(1, 0, " ⠿ reviewer ", Style::default());
    legacy_header.set_string(16, 0, " ⠿ ", Style::default());
    pane_surface.frame = FrameData::from_ratatui_buffer_with_hyperlinks(&legacy_header, None, &[]);
    pane_surface.panes[0].rect = SurfaceRect {
        x: 0,
        y: 0,
        width: 20,
        height: 8,
    };
    pane_surface.panes[0].inner_rect = SurfaceRect {
        x: 1,
        y: 1,
        width: 18,
        height: 6,
    };
    let mut config = ClientShellConfig::from_config(&Config::default());
    config.top_padding = 1;
    let mut state = ClientShellState::new(config);
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(pane_surface);
    let rendered = state
        .compose(106, 20)
        .expect("named pane and new-tab button");
    let pane = state.hits.panes[0].clone();
    let rendered = rendered.to_ratatui_buffer().expect("buffer");
    let header = (pane.rect.x..pane.rect.right())
        .map(|x| rendered[(x, pane.rect.y)].symbol())
        .collect::<String>();
    assert_eq!(header.matches('⠿').count(), 1);
    assert!(header.contains("⠿ reviewer"));
    assert_ne!(rendered[(pane.rect.right() - 3, pane.rect.y)].symbol(), "⠿");
    let new_tab = state.hits.new_tab;

    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: pane.rect.x + 2,
        row: pane.rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: new_tab.x + new_tab.width / 2,
        row: new_tab.y,
        modifiers: KeyModifiers::empty(),
    })]);
    // The source title may change or leave the surface while hovering another
    // tab. Detachment must use the name captured when the drag began.
    state.snapshot.as_mut().expect("snapshot").panes[0].label = Some("changed later".into());
    let release =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: new_tab.x + new_tab.width / 2,
            row: new_tab.y,
            modifiers: KeyModifiers::empty(),
        })]);

    assert!(matches!(
        &release.actions[..],
        [ClientShellAction::Endpoint { request, .. }]
            if matches!(
                &request.method,
                crate::api::schema::Method::PaneMove(params)
                    if params.pane_id == "pane_1"
                        && matches!(
                            &params.destination,
                            crate::api::schema::PaneMoveDestination::NewTab {
                                workspace_id: None,
                                label: Some(label),
                            } if label == "reviewer"
                        )
            )
    ));
}

#[test]
fn pane_drag_keeps_dynamic_title_from_mouse_down() {
    let mut projected = snapshot();
    projected.panes[0].label = None;
    let mut pane_surface = surface();
    let mut header = Buffer::empty(Rect::new(0, 0, 20, 8));
    header.set_string(1, 0, " zsh ", Style::default());
    pane_surface.frame = FrameData::from_ratatui_buffer_with_hyperlinks(&header, None, &[]);
    pane_surface.panes[0].rect = SurfaceRect {
        x: 0,
        y: 0,
        width: 20,
        height: 8,
    };
    pane_surface.panes[0].inner_rect = SurfaceRect {
        x: 1,
        y: 1,
        width: 18,
        height: 6,
    };
    let mut config = ClientShellConfig::from_config(&Config::default());
    config.top_padding = 1;
    let mut state = ClientShellState::new(config);
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(pane_surface.clone());
    state.compose(106, 20).expect("pane header");
    let pane = state.hits.panes[0].clone();
    let plus = state.hits.new_tab;
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: pane.rect.x + 2,
        row: pane.rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    // A live title update can arrive before the first mouse movement.
    header.set_string(1, 0, " codex ", Style::default());
    pane_surface.surface_revision += 1;
    pane_surface.frame = FrameData::from_ratatui_buffer_with_hyperlinks(&header, None, &[]);
    state.set_pane_surface(pane_surface);
    assert_eq!(
        super::super::composition::pane_header_label(
            state.snapshot.as_deref().expect("snapshot"),
            state.pane_surface.as_ref().expect("surface"),
            "pane_1",
        )
        .as_deref(),
        Some("codex")
    );
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: plus.x + plus.width / 2,
        row: plus.y,
        modifiers: KeyModifiers::empty(),
    })]);
    let release =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: plus.x + plus.width / 2,
            row: plus.y,
            modifiers: KeyModifiers::empty(),
        })]);
    assert!(
        matches!(&release.actions[..], [ClientShellAction::Endpoint { request, .. }]
        if matches!(&request.method, crate::api::schema::Method::PaneMove(params)
            if params.pane_id == "pane_1" && matches!(&params.destination,
                crate::api::schema::PaneMoveDestination::NewTab { label: Some(label), .. }
                if label == "zsh")))
    );
}

#[test]
fn pane_handle_click_does_not_detach_and_highlights_new_tab() {
    let mut projected = snapshot();
    projected.panes[0].label = Some("reviewer".into());
    let mut pane_surface = surface();
    pane_surface.frame = FrameData::from_ratatui_buffer_with_hyperlinks(
        &Buffer::empty(Rect::new(0, 0, 20, 8)),
        None,
        &[],
    );
    pane_surface.panes[0].rect = SurfaceRect {
        x: 0,
        y: 0,
        width: 20,
        height: 8,
    };
    pane_surface.panes[0].inner_rect = SurfaceRect {
        x: 1,
        y: 1,
        width: 18,
        height: 6,
    };
    let mut config = ClientShellConfig::from_config(&Config::default());
    config.top_padding = 1;
    let mut state = ClientShellState::new(config);
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(pane_surface);
    state.compose(106, 20).expect("named pane");
    let pane = state.hits.panes[0].clone();
    let point = (pane.rect.x + 2, pane.rect.y);

    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: point.0,
        row: point.1,
        modifiers: KeyModifiers::empty(),
    })]);
    let frame = state.compose(106, 20).expect("pressed handle");
    let buffer = frame.to_ratatui_buffer().expect("buffer");
    assert_eq!(
        buffer[(state.hits.new_tab.x, state.hits.new_tab.y)].bg,
        super::super::composition::hover_background(&state.config.palette, false)
    );
    let release =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: point.0,
            row: point.1,
            modifiers: KeyModifiers::empty(),
        })]);

    assert!(release.actions.is_empty());
    let frame = state.compose(106, 20).expect("released handle");
    let buffer = frame.to_ratatui_buffer().expect("buffer");
    assert_ne!(
        buffer[(state.hits.new_tab.x, state.hits.new_tab.y)].bg,
        state.config.palette.accent
    );
}

#[test]
fn tab_wheel_switches_tabs_without_changing_overflow_scroll() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("tab bar");
    let tab = state.hits.tabs[0].0;

    let outcome =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: tab.x,
            row: tab.y,
            modifiers: KeyModifiers::empty(),
        })]);
    assert!(matches!(
        &outcome.actions[..],
        [ClientShellAction::Endpoint { request, .. }]
            if matches!(
                &request.method,
                crate::api::schema::Method::TabFocus(target) if target.tab_id == "tab_1"
            )
    ));
    assert_eq!(state.tab_scroll, 0);
    state.compose(106, 20).expect("tab bar after wheel");
    assert!(state.hits.tabs.iter().any(|(_, tab_id)| tab_id == "tab_1"));
}

#[test]
fn context_menu_keyboard_and_outside_click_are_client_owned() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("composed frame");
    let tab = state.hits.tabs[0].0;
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Right),
        column: tab.x + 1,
        row: tab.y,
        modifiers: KeyModifiers::empty(),
    })]);
    state.compose(106, 20).expect("tab context menu");
    let moved = state.handle_input_bytes(b"\x1b[B");
    assert!(moved.repaint);
    assert!(matches!(
        state.overlay,
        Some(ClientShellOverlay::ContextMenu(ClientContextMenuOverlay {
            highlighted: 1,
            ..
        }))
    ));
    let text = state.handle_raw_events(vec![RawInputEvent::Text(crate::input::TextCommit::new(
        "not pane input",
    ))]);
    assert!(text.requests.is_empty());
    let paste = state.handle_raw_events(vec![RawInputEvent::Paste("not pane input".into())]);
    assert!(paste.requests.is_empty());
    let outside =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 105,
            row: 19,
            modifiers: KeyModifiers::empty(),
        })]);
    assert!(outside.repaint);
    assert!(state.overlay.is_none());
}
