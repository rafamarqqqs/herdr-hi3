import {test, expect} from 'bun:test';
import {project, destination, paneName} from './navigation.js';
const snapshot = {
  workspaces:[{workspace_id:'w1'},{workspace_id:'w2'}],
  tabs:[{tab_id:'w1:t1',workspace_id:'w1'},{tab_id:'w1:t2',workspace_id:'w1'},{tab_id:'w2:t1',workspace_id:'w2'}],
  panes:[
    {pane_id:'w1:p1',terminal_id:'a',workspace_id:'w1',tab_id:'w1:t1',focused:true,label:'zsh'},
    {pane_id:'w1:p2',terminal_id:'b',workspace_id:'w1',tab_id:'w1:t2'},
    {pane_id:'w2:p1',terminal_id:'c',workspace_id:'w2',tab_id:'w2:t1'},
  ],
  agents:[{terminal_id:'b',name:'reviewer',agent_status:'idle'},{terminal_id:'b',name:'reviewer'}],
};
test('selection projects only its tabs without moving or mutating topology',()=>{
  const before=JSON.stringify(snapshot);
  expect(project(snapshot,'c').tabs.map(t=>t.tab_id)).toEqual(['w2:t1']);
  expect(JSON.stringify(snapshot)).toBe(before);
});
test('idle real agents remain, shells never become agents, duplicate views deduplicate',()=>{
  expect(project(snapshot,'a').agents).toEqual([snapshot.agents[0]]);
});
test('remembered Space selection beats server focus and missing targets are safe',()=>{
  expect(destination(snapshot,'space','w1',{w1:'w1:p2'}).terminal_id).toBe('b');
  expect(destination(snapshot,'tab','w1:t2').terminal_id).toBe('b');
  expect(destination(snapshot,'tab','missing')).toBeUndefined();
  expect(project({},'gone').pane).toBeUndefined();
});
test('explicit pane name is preserved',()=>expect(paneName(snapshot.panes[0])).toBe('zsh'));
test('shared terminals retain selected pane context instead of jumping to first view',()=>{
  const shared={...snapshot,panes:[...snapshot.panes,{pane_id:'w2:p2',terminal_id:'a',workspace_id:'w2',tab_id:'w2:t1'}]};
  expect(project(shared,'a','w2:p2').pane.workspace_id).toBe('w2');
  expect(project(shared,'a','removed').pane).toBeUndefined();
});
test('agent rows are grouped by Space without mutating server order',()=>{
  const agents=[{terminal_id:'c',workspace_id:'w2'},{terminal_id:'a',workspace_id:'w1'}];
  const data={...snapshot,agents};
  expect(project(data,'a').agents.map(a=>a.terminal_id)).toEqual(['a','c']);
  expect(agents.map(a=>a.terminal_id)).toEqual(['c','a']);
});
