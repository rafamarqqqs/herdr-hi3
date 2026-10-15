const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
// One attachment owns one non-reusable backend lease; no global focused target.
export class TerminalSession {
  constructor(invoke,onFrame,onError){
    Object.assign(this,{invoke,onFrame,onError,closed:false,active:false,id:null,bytes:0,count:0,writes:Promise.resolve(),pumpTask:Promise.resolve()});
  }
  open(terminalId,cols,rows){
    this.opening=this.invoke('connect',{terminalId,cols,rows}).then(info=>{
      this.id=info.connection_id;
      if(!this.closed){this.active=true;this.pumpTask=this.pump();}
      return info;
    });
    return this.opening;
  }
  send(command,args){
    if(!this.active||this.closed)return;
    const size=new TextEncoder().encode(args.text??'').length;
    if(this.bytes+size>262144||this.count>=256){this.fail('Terminal input queue full; reconnect this view.');return;}
    this.bytes+=size;this.count++;
    const connectionId=this.id;
    this.writes=this.writes.then(()=>{
      if(!this.closed&&this.active)return this.invoke(command,{...args,connectionId});
    }).catch(e=>this.fail(e)).finally(()=>{this.bytes-=size;this.count--;});
  }
  fail(error){this.active=false;if(!this.closed)this.onError(error);}
  async pump(){
    try{
      while(this.active&&!this.closed){
        const frame=await this.invoke('read_frame',{connectionId:this.id});
        if(this.closed)return;
        if(frame?.closed){this.fail(frame.reason||'Terminal disconnected');return;}
        if(frame)await this.onFrame(frame);
        else await pause(16);
      }
    }catch(e){this.fail(e);}
  }
  close(){
    if(this.closing)return this.closing;
    this.closed=true;this.active=false;
    this.closing=(async()=>{
      try{await this.opening;}catch{/* Failed attachment owns no lease. */}
      await this.writes;await this.pumpTask;
      if(this.id)await this.invoke('disconnect',{connectionId:this.id});
    })();
    return this.closing;
  }
}
