import {test,expect} from 'bun:test';
import {createDemoRuntime} from './demo-runtime.js';
import {visiblePanes} from './layout.js';
test('demo covers all statuses and excludes ordinary shells',async()=>{
  const invoke=createDemoRuntime(),snapshot=await invoke('navigation');
  expect(new Set(snapshot.agents.map(a=>a.agent_status))).toEqual(new Set(['working','idle','blocked','done','unknown']));
  expect(snapshot.panes).toHaveLength(7);expect(snapshot.agents).toHaveLength(6);
  for(const tab of snapshot.tabs)expect(visiblePanes(snapshot,tab.tab_id).length).toBe(tab.pane_count);
  snapshot.agents.length=0;expect((await invoke('navigation')).agents).toHaveLength(6);
});
test('demo routes no operations to a real runtime and rejects mutations',async()=>{
  const invoke=createDemoRuntime(),info=await invoke('initialize');
  const lease=await invoke('connect',{terminalId:info.terminal_id});
  const args={connectionId:lease.connection_id};
  expect(atob((await invoke('read_frame',args)).bytes)).toContain('VISUAL DEMO');
  await invoke('input',{...args,text:'echo demo'});
  await expect(invoke('split_pane',{})).rejects.toThrow('unavailable');
  await invoke('disconnect',args);await expect(invoke('read_frame',args)).rejects.toThrow('detached');
});
test('new Space uses the selected mock directory and creates no agent',async()=>{
  const invoke=createDemoRuntime(),before=await invoke('navigation');
  const source=before.panes[3];
  const result=await invoke('create_space',{paneId:source.pane_id});
  expect(result.root_pane.cwd).toBe(source.cwd);
  expect(result.workspace.label).toBe('quality-checks');
  const after=await invoke('navigation');expect(after.agents).toHaveLength(before.agents.length);
  expect(visiblePanes(after,result.tab.tab_id)).toHaveLength(1);
});
