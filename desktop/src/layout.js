export function visiblePanes(snapshot,tabId){
  const panes=(snapshot.panes??[]).filter(p=>p.tab_id===tabId);
  const layout=(snapshot.layouts??[]).find(l=>l.tab_id===tabId);
  if(!layout){
    if(panes.length===1)return [{pane:panes[0],rect:{x:0,y:0,width:1,height:1}}];
    throw new Error('This tab has no layout snapshot; refusing to invent a split.');
  }
  const area=layout.area;
  if(layout.zoomed)throw new Error('Zoomed tabs are not supported in this candidate yet.');
  if(!area||area.width<=0||area.height<=0)throw new Error('Invalid tab geometry');
  if(layout.panes.length>32)throw new Error('At most 32 panes can be displayed.');
  if(layout.panes.length!==panes.length||panes.some(p=>!layout.panes.some(entry=>entry.pane_id===p.pane_id)))throw new Error('Incomplete tab layout');
  const seen=new Set();
  const ids=new Set(),rects=[];
  return layout.panes.map(entry=>{
    const pane=panes.find(p=>p.pane_id===entry.pane_id),r=entry.rect;
    if(!pane||!r||![r.x,r.y,r.width,r.height,area.x,area.y,area.width,area.height].every(Number.isFinite)||r.width<=0||r.height<=0||r.x<area.x||r.y<area.y||r.x+r.width>area.x+area.width||r.y+r.height>area.y+area.height)throw new Error('Incomplete pane geometry');
    if(ids.has(pane.pane_id)||rects.some(other=>r.x<other.x+other.width&&r.x+r.width>other.x&&r.y<other.y+other.height&&r.y+r.height>other.y))throw new Error('Overlapping pane geometry');
    ids.add(pane.pane_id);rects.push(r);
    if(seen.has(pane.terminal_id))throw new Error('Simultaneous shared views of one terminal are not supported yet.');
    seen.add(pane.terminal_id);
    return {pane,rect:{x:(r.x-area.x)/area.width,y:(r.y-area.y)/area.height,width:r.width/area.width,height:r.height/area.height}};
  });
}
