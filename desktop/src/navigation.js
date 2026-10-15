// Pure projection: selection never sends a move, merge, split, or server-focus action.
export function project(snapshot, terminalId, paneId = null) {
  const spaces = snapshot.workspaces ?? [];
  const tabs = snapshot.tabs ?? [];
  const panes = snapshot.panes ?? [];
  const pane = panes.find(p => paneId ? p.pane_id === paneId : p.terminal_id === terminalId);
  const seen = new Set();
  const agents = (snapshot.agents ?? []).filter(a => {
    if (seen.has(a.terminal_id)) return false;
    seen.add(a.terminal_id);
    return true;
  }).sort((a,b)=>spaces.findIndex(s=>s.workspace_id===a.workspace_id)-spaces.findIndex(s=>s.workspace_id===b.workspace_id));
  return { spaces, agents, pane,
    tabs: tabs.filter(t => t.workspace_id === pane?.workspace_id),
    panes: panes.filter(p => p.tab_id === pane?.tab_id),
  };
}
export function destination(snapshot, kind, id, remembered = {}) {
  const matches = (snapshot.panes ?? []).filter(p => p[kind === 'space' ? 'workspace_id' : 'tab_id'] === id);
  return matches.find(p => p.pane_id === remembered[id])
    ?? matches.find(p => p.focused) ?? matches[0];
}
export function paneName(pane) {
  return pane?.label || pane?.title || pane?.terminal_title_stripped || pane?.agent || 'terminal';
}
