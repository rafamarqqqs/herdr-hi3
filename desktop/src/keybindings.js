const directions={ArrowLeft:'left',ArrowDown:'down',ArrowUp:'up',ArrowRight:'right',KeyJ:'left',KeyK:'down',KeyL:'up',Semicolon:'right'};
export function paneBinding(event,{resize=false,modal=false}={}){
  if(modal||event.isComposing||event.keyCode===229)return null;
  const mod=event.metaKey&&event.altKey&&!event.ctrlKey;
  if(resize&&!event.metaKey&&!event.altKey&&!event.ctrlKey){
    if(event.code==='Escape'||event.code==='Enter')return {type:'end-resize'};
    if(directions[event.code])return {type:'resize',direction:directions[event.code]};
    return {type:'consume'};
  }
  if(!mod)return null;
  const direction=directions[event.code];
  if(direction)return {type:event.shiftKey?'move':'focus',direction};
  if(event.shiftKey)return null;
  if(event.code==='KeyH'||event.code==='KeyV')return {type:'orientation',direction:event.code==='KeyH'?'right':'down'};
  if(event.code==='Enter')return {type:'split'};
  if(event.code==='KeyF')return {type:'zoom'};
  if(event.code==='KeyR')return {type:'begin-resize'};
  return null;
}
// Focus remains client-local. Favor neighbors with overlap on the perpendicular axis.
export function paneNeighbor(entries,paneId,direction){
  const source=entries.find(entry=>entry.pane.pane_id===paneId);if(!source)return null;
  const horizontal=direction==='left'||direction==='right',positive=direction==='right'||direction==='down';
  const axis=horizontal?'x':'y',cross=horizontal?'y':'x',size=horizontal?'width':'height',crossSize=horizontal?'height':'width';
  const a=source.rect,center=a[axis]+a[size]/2;
  return entries.filter(entry=>entry!==source).map(entry=>{
    const b=entry.rect,delta=(b[axis]+b[size]/2-center)*(positive?1:-1);
    const overlap=Math.min(a[cross]+a[crossSize],b[cross]+b[crossSize])-Math.max(a[cross],b[cross]);
    return {entry,delta,overlap,cross:Math.abs(a[cross]+a[crossSize]/2-b[cross]-b[crossSize]/2)};
  }).filter(candidate=>candidate.delta>0&&candidate.overlap>0)
    .sort((a,b)=>a.delta-b.delta||a.cross-b.cross)[0]?.entry.pane??null;
}
