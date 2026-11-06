const {chromium}=require('playwright');
(async()=>{const b=await chromium.launch({args:['--use-gl=angle','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
const jobs=[['anat-frente','view=full&rot=0',1000,1600],['anat-cardio','view=busto&rot=0.25&focus=cardio',1000,1000],['anat-abdome','view=abdome&rot=-0.2&focus=abdome',1000,1000]];
for(const [name,q,w,h] of jobs){const p=await b.newPage({viewport:{width:w,height:h}});p.on('pageerror',e=>console.log('ERR',e.message));
await p.goto(`http://localhost:8765/anat.html?${q}&w=${w}&h=${h}`);await p.waitForFunction(()=>window.__done,null,{timeout:280000});
console.log(name,JSON.stringify(await p.evaluate(()=>window.__pts)));
await p.locator('canvas').screenshot({path:`out-${name}.png`,omitBackground:true});await p.close();}
await b.close();})();
