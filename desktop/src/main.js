import {invoke as realInvoke} from '@tauri-apps/api/core';
import './style.css';
import {project,destination,paneName} from './navigation.js';
import {visiblePanes} from './layout.js';
import {TerminalView} from './terminal-view.js';
import {createDemoRuntime} from './demo-runtime.js';
import {attachMenu} from './menu.js';
import {paneBinding,paneNeighbor} from './keybindings.js';

let invoke=realInvoke,demo=false,switching=false,initializing=false,rebuilding=false,creatingSpace=false,modeEpoch=0;
let realSelection=null;
let resizeMode=false,zoomPaneId=null,paneActionPending=false;
const splitOrientations=new Map();
function interactionHint(){
  const direction=splitOrientations.get(selectedPaneId)??'right';
  document.querySelector('#interaction-mode').textContent=resizeMode?'RESIZE · arrows · Esc exits':zoomPaneId?'PANE ZOOM · ⌘⌥F exits':`next split: ${direction} · ⌘⌥Enter`;
}
function endResize(){resizeMode=false;interactionHint();}

const status=document.querySelector('#status');
const error=document.querySelector('#error');
const reconnect=document.querySelector('#reconnect');
let connected=false,terminalId=null,selectedPaneId=null,snapshot={},fontSize=13;
const remembered={},views=new Map();
let layoutQueue=Promise.resolve(),layoutEpoch=0,splitPending=false,focusRequested=false;
function notice(e){error.hidden=false;error.textContent=String(e);}
async function start(preferred=null){
  initializing=true;
  try{
    const info=await invoke('initialize');terminalId=info.terminal_id;
    if(preferred){terminalId=preferred.terminalId;selectedPaneId=preferred.paneId;}
    document.querySelector('#location').textContent='LOCAL · '+info.data_dir;
    connected=true;status.textContent=demo?'DEMO · simulated agents':'local · test server';
    if(!await refresh())throw new Error('Session snapshot unavailable');
    await syncVisible(true);
  }catch(e){connected=false;notice(e);status.textContent='Disconnected';}
  finally{initializing=false;}
}
function syncVisible(focus){
  focusRequested ||= focus;
  const generation=++layoutEpoch;
  const selected=project(snapshot,terminalId,selectedPaneId).pane;
  let visible;
  try{visible=selected?visiblePanes(snapshot,selected.tab_id):[];}
  catch(e){notice(e);visible=[];}
  if(zoomPaneId){const entry=visible.find(entry=>entry.pane.pane_id===zoomPaneId);visible=entry?[{...entry,rect:{x:0,y:0,width:1,height:1}}]:visible;}
  const wanted=new Set(visible.map(v=>v.pane.pane_id));
  for(const [id,view] of views)if(!wanted.has(id))view.setInput(false);
  layoutQueue=layoutQueue.then(async()=>{
    if(generation!==layoutEpoch)return;
    for(const [id,view] of views){
      if(!wanted.has(id)){await view.close();views.delete(id);}
    }
    if(generation!==layoutEpoch)return;
    for(const entry of visible){
      if(generation!==layoutEpoch)return;
      let view=views.get(entry.pane.pane_id);
      if(!view){
        view=new TerminalView(entry.pane,invoke,selectPane,splitPane,fontSize);
        views.set(entry.pane.pane_id,view);document.querySelector('#panes').append(view.element);
        view.update(entry.pane,entry.rect,entry.pane.pane_id===selected?.pane_id);
        await view.open();
        if(generation!==layoutEpoch){view.setInput(false);return;}
      }
      view.setInput(true);
      view.update(entry.pane,entry.rect,entry.pane.pane_id===selected?.pane_id);
    }
    if(focusRequested&&generation===layoutEpoch){if(!document.querySelector('dialog[open]'))views.get(selected?.pane_id)?.focus();focusRequested=false;}
  }).catch(notice);
  return layoutQueue;
}
async function rebuildViews(){
  rebuilding=true;
  try{
  ++layoutEpoch;
  layoutQueue=layoutQueue.then(async()=>{for(const view of views.values())await view.close();views.clear();});
  await layoutQueue;
  connected=Boolean(await refresh());
  if(!connected)throw new Error('Could not reconnect to the current mode');
  status.textContent=demo?'DEMO · simulated agents':'local · test server';
  await syncVisible(true);
  }finally{rebuilding=false;}
}
async function splitPane(pane,direction){
  if(splitPending||switching||paneActionPending||creatingSpace||creatingTab)return;splitPending=true;
  try{
    const result=await invoke('split_pane',{paneId:pane.pane_id,direction});
    await freshSnapshot();selectPane(result.pane);
  }catch(e){notice(e);}finally{splitPending=false;}
}
const $=selector=>document.querySelector(selector);
function button(label, selected, action) {
  const node=document.createElement('button'); node.textContent=label;
  node.setAttribute('aria-selected',String(selected));
  node.onclick=action; return node;
}
function row(name, detail, state, selected, action) {
  const node=button('',selected,action); node.className='row'; node.title=`${name} · ${detail}`;
  node.setAttribute('aria-label',`${name}, ${detail}`);
  const dot=document.createElement('span'); dot.className=`dot ${['working','blocked','done','idle'].includes(state)?state:'unknown'}`; dot.textContent=state==='idle'?'○':state==='unknown'?'·':'●'; dot.setAttribute('aria-hidden','true');
  const text=document.createElement('span'); text.className='text';
  const title=document.createElement('span'); title.className='name'; title.textContent=name;
  const small=document.createElement('small'); small.textContent=detail;
  text.append(title,small); node.append(dot,text); return node;
}
function choose(kind,id){
  const target=destination(snapshot,kind,id,remembered);
  if(target) selectPane(target);
}
function selectPane(pane,focus=true){
  if(switching)return;
  if(selectedPaneId!==pane.pane_id){resizeMode=false;zoomPaneId=null;}
  selectedPaneId=pane.pane_id;terminalId=pane.terminal_id;
  interactionHint();
  renderNavigation();void syncVisible(focus);
}
// Keep button identity across polls: output/status updates must not eat a touchpad click.
function keyed(key,node){node.dataset.key=key;return node;}
function reconcile(container,nodes){
  const old=new Map([...container.children].map(node=>[node.dataset.key,node]));
  const keep=new Set();
  nodes.forEach((next,index)=>{
    const node=old.get(next.dataset.key) ?? next;keep.add(node);
    if(node!==next){
      for(const attribute of [...node.attributes]) if(!next.hasAttribute(attribute.name)) node.removeAttribute(attribute.name);
      for(const attribute of next.attributes) node.setAttribute(attribute.name,attribute.value);
      node.onclick=next.onclick;
      if(node.innerHTML!==next.innerHTML) node.replaceChildren(...next.childNodes);
    }
    if(container.children[index]!==node) container.insertBefore(node,container.children[index] ?? null);
  });
  for(const node of [...container.children]) if(!keep.has(node)) node.remove();
}
let signature='';
let creatingTab=false;
function renderNavigation(){
  const view=project(snapshot,terminalId,selectedPaneId);
  if(view.pane){
    remembered[view.pane.workspace_id]=view.pane.pane_id; remembered[view.pane.tab_id]=view.pane.pane_id;
  }
  const next=JSON.stringify(view);
  if(next===signature) return;
  signature=next;
  reconcile($('#spaces'),view.spaces.map(space=>keyed(space.workspace_id,row(space.label,space.agent_status,space.agent_status,space.workspace_id===view.pane?.workspace_id,()=>choose('space',space.workspace_id)))));
  reconcile($('#agents'),view.agents.map(agent=>{
    const space=view.spaces.find(s=>s.workspace_id===agent.workspace_id);
    const tab=(snapshot.tabs??[]).find(t=>t.tab_id===agent.tab_id);
    return keyed(agent.terminal_id,row(`${agent.name || agent.display_agent || agent.agent || 'agent'} · ${agent.agent_status}`,`${space?.label??'Space'} · ${tab?.label??'tab'}`,agent.agent_status,agent.terminal_id===terminalId,()=>selectPane(agent)));
  }));
  if(!view.agents.length){const empty=document.createElement('p');empty.className='empty';empty.textContent='No agents running';$('#agents').append(empty);}
  const tabNodes=view.tabs.map(tab=>keyed(tab.tab_id,button(tab.label,tab.tab_id===view.pane?.tab_id,()=>choose('tab',tab.tab_id))));
  if(view.pane){
    const add=button('+',false,async event=>{
      if(creatingTab||switching||paneActionPending||splitPending||creatingSpace) return;
      creatingTab=true;const control=event.currentTarget;control.disabled=true;
      try{const result=await invoke('create_tab',{workspaceId:view.pane.workspace_id,paneId:view.pane.pane_id});await freshSnapshot();selectPane(result.root_pane);}catch(e){notice(e);}finally{creatingTab=false;control.disabled=false;}
    });add.setAttribute('aria-label','New tab');tabNodes.push(keyed(`add-${view.pane.workspace_id}`,add));
    add.disabled=creatingTab||demo;
    if(demo)add.title='Creation is disabled in the visual demo';
  }
  reconcile($('#tabs'),tabNodes);

}
let refreshing=null;
async function freshSnapshot(){if(refreshing)await refreshing;if(!await refresh())throw new Error('Could not refresh the session');}
function refresh(){
  if(refreshing) return refreshing;
  const generation=modeEpoch;
  refreshing=invoke('navigation').then(data=>{if(generation!==modeEpoch)return false;snapshot=data;renderNavigation();void syncVisible(false);return true;}).catch(e=>{if(generation===modeEpoch)notice(e);return false;}).finally(()=>{refreshing=null;});
  return refreshing;
}
async function poll(){if(connected&&!switching) await refresh();setTimeout(poll,2000);}
$('#collapse').addEventListener('click',()=>{
  const collapsed=$('#sidebar').classList.toggle('collapsed');
  $('#collapse').setAttribute('aria-expanded',String(!collapsed));
  $('#collapse').setAttribute('aria-label',collapsed?'Expand sidebar':'Collapse sidebar');
  $('#collapse').textContent=collapsed?'»':'«';
});
attachMenu($('#menu'),$('#menu-dialog'));
$('#menu').addEventListener('click',endResize);
$('#keybinds').addEventListener('click',()=>{$('#menu-dialog').close();endResize();$('#keybinds-dialog').showModal();});
$('#settings').addEventListener('click',()=>{$('#menu-dialog').close();$('#settings-dialog').showModal();});
$('#font-size').addEventListener('input',event=>{
  fontSize=Number(event.target.value);for(const view of views.values())view.setFont(fontSize);$('#font-value').textContent=`${fontSize} px`;
});
$('#new-space').addEventListener('click',async()=>{
  if(switching||creatingSpace||initializing||paneActionPending||splitPending||creatingTab)return;
  const pane=project(snapshot,terminalId,selectedPaneId).pane;
  if(!pane){notice('Select a pane before creating a Space.');return;}
  creatingSpace=true;$('#new-space').disabled=true;
  try{
    const result=await invoke('create_space',{paneId:pane.pane_id});
    await freshSnapshot();selectPane(result.root_pane);
  }catch(e){notice(e);}finally{creatingSpace=false;$('#new-space').disabled=false;}
});
reconnect.addEventListener('click',async()=>{
  if(switching||rebuilding||initializing)return;
  $('#menu-dialog').close();reconnect.disabled=true;
  try{await rebuildViews();}catch(e){notice(e);}finally{reconnect.disabled=false;}
});
$('#demo-toggle').addEventListener('click',async()=>{
  if(switching||initializing||rebuilding||creatingSpace||creatingTab||splitPending||paneActionPending){notice('Wait for the current operation before switching modes.');return;}
  resizeMode=false;zoomPaneId=null;splitOrientations.clear();interactionHint();
  switching=true;connected=false;++modeEpoch;++layoutEpoch;
  const previousSelection={terminalId,paneId:selectedPaneId};
  for(const view of views.values())view.setInput(false);
  document.body.inert=true;
  try{
    if(refreshing)await refreshing;
    await layoutQueue;
    for(const view of views.values())await view.close();
    views.clear();
    if(!demo)realSelection=previousSelection;
    demo=!demo;invoke=demo?createDemoRuntime():realInvoke;
    snapshot={};terminalId=null;selectedPaneId=null;signature='';focusRequested=false;
    for(const key of Object.keys(remembered))delete remembered[key];
    error.hidden=true;$('#menu-dialog').close();
    $('#demo-toggle').textContent=demo?'return to test server':'agent states demo';
    $('#new-space').disabled=false;
    document.body.classList.toggle('demo-mode',demo);
    await start(demo?null:realSelection);
  }catch(e){notice(e);status.textContent='Mode switch failed · reconnect to retry';}
  finally{
    switching=false;document.body.inert=false;
    const pane=project(snapshot,terminalId,selectedPaneId).pane;
    views.get(pane?.pane_id)?.focus();
  }
});
window.addEventListener('blur',endResize);
document.addEventListener('keydown',async event=>{
  if(event.defaultPrevented||switching||initializing||rebuilding)return;
  const binding=paneBinding(event,{resize:resizeMode,modal:Boolean(document.querySelector('dialog[open]'))});
  if(!binding)return;
  event.preventDefault();event.stopImmediatePropagation();
  if(event.repeat&&!['focus','resize','consume'].includes(binding.type))return;
  if(binding.type==='end-resize'){endResize();return;}
  if(binding.type==='consume')return;
  const pane=project(snapshot,terminalId,selectedPaneId).pane;if(!pane)return;
  let ownsMutation=false;
  try{
    if(binding.type==='orientation'){splitOrientations.set(pane.pane_id,binding.direction);selectedPaneId=pane.pane_id;interactionHint();return;}
    if(binding.type==='begin-resize'){resizeMode=true;interactionHint();return;}
    if(binding.type==='zoom'){zoomPaneId=zoomPaneId?null:pane.pane_id;interactionHint();await syncVisible(true);return;}
    if(binding.type==='split'){zoomPaneId=null;await splitPane(pane,splitOrientations.get(pane.pane_id)??'right');return;}
    const neighbor=paneNeighbor(visiblePanes(snapshot,pane.tab_id),pane.pane_id,binding.direction);
    if(binding.type==='focus'){if(neighbor)selectPane(neighbor);return;}
    if(paneActionPending||splitPending||creatingSpace||creatingTab)return;
    if(binding.type==='move'&&!neighbor)return;
    paneActionPending=true;ownsMutation=true;zoomPaneId=null;interactionHint();
    await invoke('pane_action',{paneId:pane.pane_id,action:binding.type,direction:binding.direction,targetPaneId:neighbor?.pane_id??null});
    await freshSnapshot();
  }catch(e){notice(e);}finally{if(ownsMutation)paneActionPending=false;}
},true);
interactionHint();
void start();
void poll();
