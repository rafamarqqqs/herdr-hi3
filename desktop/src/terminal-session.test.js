import {test,expect} from 'bun:test';
import {TerminalSession} from './terminal-session.js';
test('closing during attachment releases the late lease without rendering',async()=>{
  let resolve;const calls=[];
  const invoke=(command,args)=>{calls.push([command,args]);return command==='connect'?new Promise(r=>resolve=r):Promise.resolve(null);};
  const session=new TerminalSession(invoke,()=>{throw Error('must not render');},()=>{});
  const opened=session.open('first',80,24);const closed=session.close();
  resolve({connection_id:'lease1'});await opened;await closed;
  expect(calls.map(c=>c[0])).toEqual(['connect','disconnect']);
  expect(calls[1][1]).toEqual({connectionId:'lease1'});
});
test('siblings route input and detach independently',async()=>{
  const calls=[];
  const invoke=async(command,args)=>{calls.push([command,args]);if(command==='connect')return {connection_id:args.terminalId};return null;};
  const a=new TerminalSession(invoke,()=>{},()=>{}),b=new TerminalSession(invoke,()=>{},()=>{});
  await a.open('A',80,24);await b.open('B',40,24);
  a.send('input',{text:'a'});b.send('input',{text:'b'});await a.writes;await b.writes;
  await a.close();b.send('resize',{cols:60,rows:20});await b.writes;await b.close();
  expect(calls.filter(c=>c[0]==='input').map(c=>c[1].connectionId)).toEqual(['A','B']);
  expect(calls.find(c=>c[0]==='resize')[1].connectionId).toBe('B');
});
test('one failed stream does not disable its sibling',async()=>{
  const errors=[];
  const invoke=async(command,args)=>{
    if(command==='connect')return {connection_id:args.terminalId};
    if(command==='read_frame'&&args.connectionId==='bad')throw Error('failed');return null;
  };
  const a=new TerminalSession(invoke,()=>{},e=>errors.push(e)),b=new TerminalSession(invoke,()=>{},()=>{});
  await a.open('bad',80,24);await b.open('good',80,24);await a.pumpTask;
  expect(a.active).toBe(false);expect(b.active).toBe(true);expect(errors).toHaveLength(1);
  await a.close();await b.close();
});
