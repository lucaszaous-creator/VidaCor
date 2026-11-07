const {chromium}=require('playwright');const fs=require('fs');
const dir=process.argv[2];const css=fs.readFileSync(process.argv[3],'utf8');
(async()=>{const b=await chromium.launch();const p=await b.newPage({viewport:{width:+process.argv[4],height:900}});
await p.route('**/_blob/079f870cfe8a03ec36101026af3bba84',r=>r.fulfill({body:css,contentType:'text/css'}));
await p.route('**/_blob/**',r=>r.fulfill({body:'',status:200}));
await p.route('**/support.js',r=>r.fulfill({body:'',contentType:'text/javascript'}));
for(const f of fs.readdirSync(dir).filter(f=>f.endsWith('.dc.html'))){
 await p.goto('file://'+dir+'/'+f);await p.waitForTimeout(50);
 const r=await p.evaluate(()=>{const out=[];const de=document.documentElement;if(de.scrollWidth>de.clientWidth+2)out.push('page '+de.scrollWidth);
 document.querySelectorAll('.scroll-x, [style*="overflow"]').forEach(e=>{if(e.scrollWidth>e.clientWidth+2&&getComputedStyle(e).overflowX!='hidden')out.push((e.className||e.tagName)+' '+e.scrollWidth+'>'+e.clientWidth)});return out});
 if(r.length)console.log(f,JSON.stringify(r));}
await b.close();})();
