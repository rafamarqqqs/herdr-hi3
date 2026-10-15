import {test,expect} from 'bun:test';
import {paneBinding,paneNeighbor} from './keybindings.js';
const mod={metaKey:true,altKey:true};
test('only explicit Mod combinations are captured; IME/dialogs are untouched',()=>{
  expect(paneBinding({code:'KeyJ'})).toBeNull();
  expect(paneBinding({...mod,code:'KeyJ'})).toEqual({type:'focus',direction:'left'});
  expect(paneBinding({...mod,code:'KeyJ',isComposing:true})).toBeNull();
  expect(paneBinding({...mod,code:'KeyJ'},{modal:true})).toBeNull();
  expect(paneBinding({metaKey:true,code:'KeyC'})).toBeNull();
});
test('i3 home row, shift movement and deferred split orientation',()=>{
  expect(paneBinding({...mod,code:'KeyK',shiftKey:true})).toEqual({type:'move',direction:'down'});
  expect(paneBinding({...mod,code:'Semicolon'}).direction).toBe('right');
  expect(paneBinding({...mod,code:'KeyH'})).toEqual({type:'orientation',direction:'right'});
  expect(paneBinding({...mod,code:'Enter'}).type).toBe('split');
});
test('resize mode consumes typing and exits explicitly',()=>{
  expect(paneBinding({code:'ArrowRight'},{resize:true}).type).toBe('resize');
  expect(paneBinding({code:'KeyA'},{resize:true}).type).toBe('consume');
  expect(paneBinding({code:'Escape'},{resize:true}).type).toBe('end-resize');
});
test('directional focus remains inside the tab geometry',()=>{
  const entries=[{pane:{pane_id:'a'},rect:{x:0,y:0,width:.5,height:1}},{pane:{pane_id:'b'},rect:{x:.5,y:0,width:.5,height:.5}},{pane:{pane_id:'c'},rect:{x:.5,y:.5,width:.5,height:.5}}];
  expect(paneNeighbor(entries,'b','down').pane_id).toBe('c');
  expect(paneNeighbor(entries,'c','left').pane_id).toBe('a');
  expect(paneNeighbor(entries,'a','left')).toBeNull();
});
