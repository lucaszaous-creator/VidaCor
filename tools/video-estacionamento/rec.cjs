const {chromium}=require('playwright');
(async()=>{const b=await chromium.launch();const p=await b.newPage({viewport:{width:1920,height:1080}});
p.on('pageerror',e=>console.log('ERR',e.message));
await p.goto('file://'+__dirname+'/video.html');await p.waitForTimeout(1500);
const starts=[0,5.0,13.6,18.3,24.0,30.8,36.6,40.0,46.4,52.1,55.6,63.1],TOTAL=69.5,FPS=30;
await p.evaluate(([s,t])=>window.__buildFixed(s,t),[starts,TOTAL]);
const only=process.argv[2];
const N=Math.round(TOTAL*FPS);
for(let i=0;i<N;i++){const t=i/FPS; if(only && !only.split(',').map(Number).some(x=>Math.abs(x-t)<1e-6)) continue;
 await p.evaluate(t=>window.__render(t),t);
 await p.screenshot({path:`frames/f${String(i).padStart(5,'0')}.jpg`,type:'jpeg',quality:92});}
await b.close();})();
