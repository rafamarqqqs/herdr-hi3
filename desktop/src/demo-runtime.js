// In-memory visual fixture. No dependency on Tauri, a shell, or the real runtime.
export function createDemoRuntime(){
  const workspaces=[
    {workspace_id:'demo:w1',label:'interface-lab',tab_count:4,agent_status:'working'},
    {workspace_id:'demo:w2',label:'quality-checks',tab_count:2,agent_status:'blocked'},
    {workspace_id:'demo:w3',label:'quiet-space',tab_count:1,agent_status:'idle'},
  ];
  const definitions=[
    ['builder','working',0,'Building the sidebar preview.'],
    ['reviewer','blocked',0,'Waiting for approval. No real approval is requested.'],
    ['planner','idle',0,'Ready for the next task.'],
    ['test-runner','done',1,'All simulated checks have finished.'],
    ['observer','unknown',1,'Agent present; activity could not be classified.'],
    ['paused-agent','idle',2,'Idle agents remain visible.'],
    ['shell',null,0,'A plain shell is not listed in Agents.'],
  ];
  const tabs=[],panes=[],agents=[],layouts=[];
  definitions.forEach(([name,state,spaceIndex,description],index)=>{
    const workspace_id=workspaces[spaceIndex].workspace_id;
    const tab_id=index===1?'demo:t0':`demo:t${index}`;
    if(index!==1)tabs.push({tab_id,workspace_id,label:name,pane_count:index===0?2:1});
    const pane={pane_id:`demo:p${index}`,terminal_id:`demo:term${index}`,workspace_id,tab_id,label:name,title:name,cwd:`/demo/${workspaces[spaceIndex].label}`,agent_status:state??'idle'};
    panes.push(pane);
    if(state)agents.push({...pane,name,agent:'mock',description});
  });
  for(const tab of tabs){
    const members=panes.filter(p=>p.tab_id===tab.tab_id);
    layouts.push({tab_id:tab.tab_id,zoomed:false,area:{x:0,y:0,width:100,height:50},panes:members.map((p,index)=>({pane_id:p.pane_id,rect:{x:index*50,y:0,width:members.length===2?49:100,height:50}}))});
  }
  for(const space of workspaces)space.tab_count=tabs.filter(t=>t.workspace_id===space.workspace_id).length;
  const snapshot={workspaces,tabs,panes,agents,layouts};
  const leases=new Map();let next=0;
  return async function invoke(command,args={}){
    if(command==='initialize')return {terminal_id:panes[0].terminal_id,data_dir:'DEMO — simulated agents, no processes'};
    if(command==='navigation')return structuredClone(snapshot);
    if(command==='create_space'){
      const source=panes.find(p=>p.pane_id===args.paneId);if(!source)throw new Error('Unknown demo pane');
      const index=panes.length,workspace_id=`demo:new-w${index}`,tab_id=`demo:t${index}`;
      const cwd=source.foreground_cwd||source.cwd,label=cwd.split('/').filter(Boolean).at(-1);
      const workspace={workspace_id,label,tab_count:1,agent_status:'idle'};
      const tab={workspace_id,tab_id,label:'shell',pane_count:1};
      const root_pane={workspace_id,tab_id,pane_id:`demo:p${index}`,terminal_id:`demo:term${index}`,label:'shell',cwd,agent_status:'idle'};
      workspaces.push(workspace);tabs.push(tab);panes.push(root_pane);
      definitions.push(['shell',null,workspaces.length-1,'A new mock Space in the selected directory.']);
      layouts.push({tab_id,zoomed:false,area:{x:0,y:0,width:100,height:50},panes:[{pane_id:root_pane.pane_id,rect:{x:0,y:0,width:100,height:50}}]});
      return structuredClone({workspace,tab,root_pane});
    }
    if(command==='connect'){
      const pane=panes.find(p=>p.terminal_id===args.terminalId);
      if(!pane)throw new Error('Unknown demo terminal');
      const connection_id=`demo-lease-${++next}`;leases.set(connection_id,{pane,dirty:true});
      return {terminal_id:pane.terminal_id,connection_id};
    }
    if(command==='disconnect'){leases.delete(args.connectionId);return;}
    if(['read_frame','input','resize','scroll'].includes(command)){
      const lease=leases.get(args.connectionId);if(!lease)throw new Error('Demo view detached');
      if(command!=='read_frame'){if(command==='resize')lease.dirty=true;return;}
      if(!lease.dirty)return null;lease.dirty=false;
      const index=panes.indexOf(lease.pane),[name,state,,description]=definitions[index];
      const message=`\x1b[2J\x1b[H\x1b[33mVISUAL DEMO\x1b[0m\r\n\r\n${name} - ${state??'shell'}\r\n\r\n${description}\r\n\r\nHover rows to compare them with selection.\r\nClick an agent, tab or Space to select it.\r\n\r\nTyping here runs no commands.\r\nReturn to the test server from menu.\r\n`;
      return {bytes:btoa(message),closed:false};
    }
    throw new Error('This action is unavailable in the visual demo. Return to the test server to use it.');
  };
}
