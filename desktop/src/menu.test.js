import {test,expect} from 'bun:test';
import {menuPosition} from './menu.js';
test('toolbar popup sits below button and stays inside the viewport',()=>{
  expect(menuPosition({top:12,bottom:42,right:100},200,140,{width:800,height:600})).toEqual({left:0,top:42});
});
test('bottom launcher opens upward like Herdr',()=>{
  expect(menuPosition({top:560,bottom:590,right:790},200,140,{width:800,height:600})).toEqual({left:590,top:420});
});
