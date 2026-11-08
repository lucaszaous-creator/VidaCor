// usage: node rshot.js <projectDir> <css> <w> <h> <outdir> file1 file2 ...
const {chromium}=require('playwright');const fs=require('fs');const path=require('path');
const [dir,css,W,H,out,...files]=process.argv.slice(2);
// Motor: o original da ferramenta de design em render/support.js (não versionado), se alguém o tiver;
// senão o motor aberto versionado em motor-dc/support.js (DEC-2026-11-08-39).
const original=path.join(__dirname,'render/support.js');const usaOriginal=fs.existsSync(original);
const rt=fs.readFileSync(usaOriginal?original:path.join(__dirname,'motor-dc/support.js'));
console.error('motor: '+(usaOriginal?'render/support.js (original, não versionado)':'motor-dc/support.js (aberto, versionado)'));
(async()=>{const b=await chromium.launch();const p=await b.newPage({viewport:{width:+W,height:+H}});
const errs=[];p.on('pageerror',e=>errs.push(e.message));p.on('console',m=>{if(m.type()==='error')errs.push(m.text())});
await p.route('**/*',r=>{const u=new URL(r.request().url());
  if(u.pathname.startsWith('/_blob/')){ if(u.pathname.includes(path.basename(css,'.css'))||u.pathname.endsWith(process.env.CSSID)) return r.fulfill({body:fs.readFileSync(css),contentType:'text/css'}); {const bf=path.join(__dirname,'blobimg',path.basename(u.pathname)); if(fs.existsSync(bf)) return r.fulfill({body:fs.readFileSync(bf),contentType:'image/png'}); return r.fulfill({status:404,body:''});}}
  if(u.pathname.endsWith('/support.js')) return r.fulfill({body:rt,contentType:'text/javascript'});
  if(u.hostname==='vc.local'){const f=path.join(dir,decodeURIComponent(u.pathname.slice(1))); if(fs.existsSync(f)) return r.fulfill({body:fs.readFileSync(f),contentType:'text/html'}); return r.fulfill({status:404,body:''});}
  if(u.hostname.includes('fonts.g')) return r.continue();
  return r.fulfill({status:404,body:''});});
for(const f of files){errs.length=0;await p.goto('http://vc.local/'+f);
 if(usaOriginal) await p.waitForTimeout(1500);
 else{await p.waitForFunction(()=>document.documentElement.getAttribute('data-dc-pronto')==='1',null,{timeout:15000}).catch(()=>errs.push('motor não terminou em 15 s'));
   await p.evaluate(()=>document.fonts.ready);}
 const m=await p.evaluate(()=>{const r={sh:document.documentElement.scrollHeight,sw:document.documentElement.scrollWidth,cortado:[],ladoalado:[]};
  document.querySelectorAll('body *').forEach(e=>{const cs=getComputedStyle(e);
    if((cs.overflowY==='hidden'||cs.overflow==='hidden')&&e.scrollHeight>e.clientHeight+4&&e.clientHeight>40) r.cortado.push((e.className||e.tagName)+' '+e.scrollHeight+'>'+e.clientHeight);
    if(e.scrollWidth>e.clientWidth+4&&(cs.overflowX==='auto'||cs.overflowX==='scroll')) r.ladoalado.push((e.className||e.tagName)+' '+e.scrollWidth+'>'+e.clientWidth);});
  r.cortado=r.cortado.slice(0,4);r.ladoalado=r.ladoalado.slice(0,4);return r;});
 await p.screenshot({path:path.join(out,f.replace('.dc.html','.png'))});
 console.log(f,JSON.stringify(m),errs.slice(0,3).join(' | '));}
await b.close();})();
