import {test,expect} from 'bun:test';
import {visiblePanes} from './layout.js';
function fixture(){return {panes:[1,2,3].map(i=>({pane_id:'p'+i,terminal_id:'t'+i,tab_id:'tab'})),layouts:[{tab_id:'tab',area:{x:20,y:5,width:100,height:80},panes:[
  {pane_id:'p1',rect:{x:20,y:5,width:49,height:80}},
  {pane_id:'p2',rect:{x:70,y:5,width:50,height:39}},
  {pane_id:'p3',rect:{x:70,y:45,width:50,height:40}},
]}]};}
test('nested panes normalize against nonzero origin and preserve gaps',()=>{
  const result=visiblePanes(fixture(),'tab');expect(result).toHaveLength(3);
  expect(result[0].rect).toEqual({x:0,y:0,width:.49,height:1});
  expect(result[2].rect).toEqual({x:.5,y:.5,width:.5,height:.5});
});
test('invalid geometry and overlap never invent a layout',()=>{
  const data=fixture();data.layouts[0].panes[1].rect.x=21;expect(()=>visiblePanes(data,'tab')).toThrow('Overlapping');
  data.layouts[0].area.width=0;expect(()=>visiblePanes(data,'tab')).toThrow('Invalid');
  expect(()=>visiblePanes({...fixture(),layouts:[]},'tab')).toThrow('no layout');
});
test('shared simultaneous views and zoom are explicit limitations',()=>{
  const data=fixture();data.panes[1].terminal_id='t1';expect(()=>visiblePanes(data,'tab')).toThrow('shared views');
  data.layouts[0].zoomed=true;expect(()=>visiblePanes(data,'tab')).toThrow('Zoomed');
});
test('incomplete and empty snapshots never silently hide panes',()=>{
  const data=fixture();data.layouts[0].panes.pop();expect(()=>visiblePanes(data,'tab')).toThrow('Incomplete');
  data.layouts[0].panes=[];expect(()=>visiblePanes(data,'tab')).toThrow('Incomplete');
});
