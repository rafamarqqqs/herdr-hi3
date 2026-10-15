export function menuPosition(launcher,width,height,viewport){
  return {left:Math.max(0,Math.min(launcher.right-width,viewport.width-width)),
    top:Math.max(0,launcher.top>=height?launcher.top-height:Math.min(launcher.bottom,viewport.height-height))};
}
export function attachMenu(launcher,menu){
  launcher.setAttribute('aria-haspopup','menu');launcher.setAttribute('aria-expanded','false');
  menu.setAttribute('role','menu');menu.setAttribute('aria-label','Menu');
  const items=()=>[...menu.querySelectorAll(':scope > button:not(:disabled)')];
  for(const item of items()){
    item.setAttribute('role','menuitem');
    item.addEventListener('pointerenter',()=>{if(menu.open)item.focus();});
  }
  launcher.addEventListener('click',()=>{
    if(menu.open){menu.close();return;}
    menu.show();launcher.setAttribute('aria-expanded','true');
    const rect=menu.getBoundingClientRect();
    const position=menuPosition(launcher.getBoundingClientRect(),rect.width,rect.height,{width:innerWidth,height:innerHeight});
    menu.style.left=`${position.left}px`;menu.style.top=`${position.top}px`;items()[0]?.focus();
  });
  menu.addEventListener('close',()=>launcher.setAttribute('aria-expanded','false'));
  // Outside click dismisses only; it must not also select a pane or create a Space.
  let consumeClick=false;
  document.addEventListener('pointerdown',event=>{
    if(!menu.open||menu.contains(event.target)||launcher.contains(event.target))return;
    event.preventDefault();event.stopImmediatePropagation();consumeClick=true;menu.close();
  },true);
  document.addEventListener('pointerup',()=>{setTimeout(()=>{consumeClick=false;},0);},true);
  document.addEventListener('click',event=>{if(consumeClick){consumeClick=false;event.preventDefault();event.stopImmediatePropagation();}},true);
  document.addEventListener('wheel',event=>{if(menu.open){event.preventDefault();event.stopImmediatePropagation();}},{capture:true,passive:false});
  document.addEventListener('paste',event=>{if(menu.open){event.preventDefault();event.stopImmediatePropagation();}},true);
  document.addEventListener('keydown',event=>{
    if(!menu.open)return;
    event.preventDefault();event.stopImmediatePropagation();
    const rows=items(),index=Math.max(0,rows.indexOf(document.activeElement));
    if(event.key==='Escape'){menu.close();launcher.focus();}
    else if(event.key==='ArrowUp')rows[Math.max(0,index-1)]?.focus();
    else if(event.key==='ArrowDown')rows[Math.min(rows.length-1,index+1)]?.focus();
    else if(event.key==='Enter'||event.key===' ')rows[index]?.click();
    else if(event.key==='Tab')rows[Math.max(0,Math.min(rows.length-1,index+(event.shiftKey?-1:1)))]?.focus();
  },true);
}
