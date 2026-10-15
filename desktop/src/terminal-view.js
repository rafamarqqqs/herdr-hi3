import {Terminal} from '@xterm/xterm';
import {FitAddon} from '@xterm/addon-fit';
import {TerminalSession} from './terminal-session.js';
import {paneName} from './navigation.js';
export class TerminalView {
  constructor(pane,invoke,select,split,fontSize){
    this.pane=pane;this.closed=false;this.inputEnabled=true;
    this.element=document.createElement('section');this.element.className='surface';
    const header=document.createElement('div');header.className='pane-title';
    this.title=document.createElement('span');this.title.className='pane-name';
    const actions=document.createElement('div');actions.className='pane-actions';
    for(const [direction,glyph] of [['right','▥'],['down','▤']]){
      const button=document.createElement('button');button.textContent=glyph;button.title=`Split ${direction}`;button.setAttribute('aria-label',`Split ${direction}`);
      button.onclick=event=>{event.stopPropagation();split(this.pane,direction);};
      button.disabled=pane.pane_id.startsWith('demo:');
      if(button.disabled)button.title='Splits are disabled in the visual demo';
      actions.append(button);
    }
    header.append(this.title,actions);
    this.host=document.createElement('div');this.host.className='terminal';
    this.error=document.createElement('div');this.error.className='pane-error';this.error.hidden=true;this.error.setAttribute('role','alert');
    this.element.append(header,this.host,this.error);
    this.element.addEventListener('pointerdown',()=>select(this.pane,false));
    header.addEventListener('click',()=>this.focus());
    this.term=new Terminal({fontFamily:'Menlo, Monaco, monospace',fontSize,cursorBlink:true,scrollback:0,allowProposedApi:true,
      theme:{background:'#282828',foreground:'#ebdbb2',cursor:'#ebdbb2',selectionBackground:'#665c5466',black:'#282828',red:'#cc241d',green:'#98971a',yellow:'#d79921',blue:'#458588',magenta:'#b16286',cyan:'#689d6a',white:'#a89984',brightBlack:'#928374',brightRed:'#fb4934',brightGreen:'#b8bb26',brightYellow:'#fabd2f',brightBlue:'#83a598',brightMagenta:'#d3869b',brightCyan:'#8ec07c',brightWhite:'#ebdbb2'}});
    this.fit=new FitAddon();this.term.loadAddon(this.fit);
    this.session=new TerminalSession(invoke,frame=>new Promise(resolve=>{
      this.term.write(Uint8Array.from(atob(frame.bytes),c=>c.charCodeAt(0)),resolve);
    }),error=>{this.error.textContent=String(error);this.error.hidden=false;});
    this.term.onData(text=>this.send('input',{text,paste:false}));
    this.term.parser.registerOscHandler(52,()=>true);this.term.parser.registerOscHandler(8,()=>true);
    this.host.addEventListener('paste',event=>{
      event.preventDefault();event.stopImmediatePropagation();
      if(event.clipboardData)this.send('input',{text:event.clipboardData.getData('text/plain'),paste:true});
    },true);
    this.host.addEventListener('wheel',event=>{
      if(event.ctrlKey||event.deltaY===0)return;event.preventDefault();
      this.send('scroll',{direction:event.deltaY<0?'up':'down',lines:3});
    },{capture:true,passive:false});
    this.term.attachCustomKeyEventHandler(event=>{
      if(event.metaKey&&event.key.toLowerCase()==='c'&&this.term.hasSelection()){
        if(event.type==='keydown')navigator.clipboard.writeText(this.term.getSelection()).catch(e=>{this.error.textContent=String(e);this.error.hidden=false;});return false;
      }
      return !(event.metaKey&&event.key.toLowerCase()==='v');
    });
    this.observer=new ResizeObserver(()=>this.resize());
  }
  async open(){
    this.term.open(this.host);this.observer.observe(this.host);this.fitSize();
    try{await this.session.open(this.pane.terminal_id,this.term.cols,this.term.rows);this.resize();}
    catch(e){this.error.textContent=String(e);this.error.hidden=false;}
  }
  fitSize(){this.fit.fit();this.term.resize(Math.min(400,Math.max(2,this.term.cols)),Math.min(200,Math.max(2,this.term.rows)));}
  resize(){clearTimeout(this.timer);this.timer=setTimeout(()=>{
    if(this.closed)return;this.fitSize();this.session.send('resize',{cols:this.term.cols,rows:this.term.rows});
  },80);}
  update(pane,rect,selected){
    this.pane=pane;this.title.textContent=paneName(pane);this.element.classList.toggle('selected',selected);
    Object.assign(this.element.style,{left:`${rect.x*100}%`,top:`${rect.y*100}%`,width:`${rect.width*100}%`,height:`${rect.height*100}%`});
  }
  focus(){if(!this.closed)this.term.focus();}
  send(command,args){if(this.inputEnabled&&!this.closed)this.session.send(command,args);}
  setInput(enabled){this.inputEnabled=enabled;this.term.options.disableStdin=!enabled;}
  setFont(size){this.term.options.fontSize=size;this.resize();}
  async close(){this.closed=true;clearTimeout(this.timer);this.observer.disconnect();await this.session.close();this.term.dispose();this.element.remove();}
}
