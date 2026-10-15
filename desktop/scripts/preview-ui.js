// Browser-only visual fixture. Never bundled into or connected to the native app.
// Run after `bun run build`; no shell, sockets, or real Herdr processes are used.
const fixture = `
const spaces=[{workspace_id:'w1',label:'workbench',tab_count:2,agent_status:'working'},{workspace_id:'w2',label:'hi3',tab_count:1,agent_status:'idle'}];
const tabs=[{workspace_id:'w1',tab_id:'w1:t1',label:'customizer'},{workspace_id:'w1',tab_id:'w1:t2',label:'zsh'},{workspace_id:'w2',tab_id:'w2:t1',label:'dev'}];
const panes=tabs.map((t,i)=>({...t,pane_id:'p'+i,terminal_id:'term'+i,title:t.label,focused:i===0}));
let lease=0;const frames=new Map();
window.__TAURI_INTERNALS__={invoke:async(command,args)=>{
 if(command==='navigation')return {workspaces:spaces,tabs,panes,agents:[{...panes[0],name:'customizer',agent:'codex',agent_status:'working'}]};
 if(command==='initialize')return {terminal_id:'term0',data_dir:'Visual fixture — no real server'};
 if(command==='connect'){const id=String(++lease);frames.set(id,true);return {terminal_id:args.terminalId,connection_id:id,data_dir:'Visual fixture — no real server'};}
 if(command==='disconnect'){frames.delete(args.connectionId);return;}
 if(command==='read_frame'){if(!frames.get(args.connectionId))return null;frames.set(args.connectionId,false);return {bytes:btoa('\\x1b[2J\\x1b[Hhi3 UI fixture — keyboard input is mocked'.replace('—','-')+'\\r\\n$ ')};}
 if(command==='input'||command==='scroll'||command==='resize')return;
 throw new Error('Visual fixture only: '+command+' is not executed');
}};`;
const root = new URL('../dist/', import.meta.url);
const server=Bun.serve({hostname:'127.0.0.1',port:0,async fetch(request){
  const path=new URL(request.url).pathname;
  if(path==='/fixture.js')return new Response(fixture,{headers:{'content-type':'text/javascript'}});
  if(path==='/'){
    const html=await Bun.file(new URL('index.html',root)).text();
    return new Response(html.replace('<script type="module"','<script src="/fixture.js"></script><script type="module"'),{headers:{'content-type':'text/html'}});
  }
  if(path==='/main.js'||path==='/main.css')return new Response(Bun.file(new URL(path.slice(1),root)));
  return new Response('Not found',{status:404});
}});
console.log('Visual fixture only:',server.url.href);
