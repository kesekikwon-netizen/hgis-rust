// 국토정보플랫폼 「공개DEM」 — 수치지형도와 같은 통로(로그인 → 신청 → 내려받기)에서 DEM 을 고른다.
// 공개 요청은 2026-11-11 실제 화면에서 확인했다: demQuery.do, downloadMapData('4','online',…).
// 계정·토큰·파일 주소는 다루지 않는다. 믿는 포털 화면 안에서만 돈다. 결과는 JSON 으로 돌려준다.
// 이 파일은 select·openForm 만 맡는다(나머지 명령은 ngii-topographic.js).
(function (input) {
  'use strict';
  const result = (state, message, extra) => Object.assign({state, message: message || ''}, extra || {});
  const visible = e => {
    if(!e || !e.getClientRects().length)return false;
    for(let node=e;node;node=node.parentElement){const s=getComputedStyle(node);
      if(s.display==='none' || s.visibility==='hidden' || Number(s.opacity)===0)return false;}
    return true;
  };
  const key = String(input.generation);
  if (input.command === 'select') {
    let state = window.__kaDemSelection;
    if (!state || state.key !== key) {
      state = window.__kaDemSelection = {key, state:'querying', started:Date.now()};
      const post = (path, values) => fetch(path, {method:'POST', credentials:'same-origin',
        headers:{'Content-Type':'application/x-www-form-urlencoded; charset=UTF-8'}, body:new URLSearchParams(values),
        signal:AbortSignal.timeout(30000)}).then(r=>{if(!r.ok)throw new Error('http');return r.json();});
      post('/ms/map/combineBufferQuery.do', {x:input.x,y:input.y,buffer:input.radius})
        .then(data=>{
          if(window.__kaDemSelection!==state)throw new Error('superseded');
          const area=data && data.items && data.items.length===1 && data.items[0];
          if(!area || typeof area.wkt!=='string' || !/^POLYGON\s*\(\(/i.test(area.wkt) ||
              area.wkt.length>100000 || ![area.minx,area.miny,area.maxx,area.maxy].every(Number.isFinite) ||
              Math.abs(area.minx-(input.x-input.radius))>1 || Math.abs(area.maxx-(input.x+input.radius))>1 ||
              Math.abs(area.miny-(input.y-input.radius))>1 || Math.abs(area.maxy-(input.y+input.radius))>1)throw new Error('scope');
          state.area=area;
          return post('/ms/map/demQuery.do',{wkt:area.wkt,minx:area.minx,miny:area.miny,maxx:area.maxx,maxy:area.maxy});
        })
        .then(data => {
          if (window.__kaDemSelection !== state) return;
          const items = data && data.items;
          if (items!==undefined && (!Array.isArray(items) || items.length>200)) throw new Error('sheets');
          // 도엽마다 가장 최근 연도 하나만 고른다.
          const latest = new Map();
          for (const row of items || []) {
            if (!/^\d{5,6}$/.test(String(row.num)) || !/^\d{4}$/.test(String(row.year)) ||
                typeof row.nam !== 'string' || !row.nam ||
                ![row.minx,row.miny,row.maxx,row.maxy].every(Number.isFinite) ||
                row.minx>=row.maxx || row.miny>=row.maxy || row.maxx<state.area.minx || row.minx>state.area.maxx ||
                row.maxy<state.area.miny || row.miny>state.area.maxy) throw new Error('product');
            const seen = latest.get(String(row.num));
            if (!seen || Number(row.year) > Number(seen.year))
              latest.set(String(row.num), {num:String(row.num), name:row.nam, year:String(row.year),
                minx:row.minx, miny:row.miny, maxx:row.maxx, maxy:row.maxy});
          }
          state.items = Array.from(latest.values());
          const cached = new Set(input.cachedNumbers || []);
          state.cached = state.items.filter(r=>cached.has(r.num)).map(r=>r.num);
          state.downloadItems = state.items.filter(r=>!cached.has(r.num));
          if(!state.items.length){state.state='unavailable';return;}
          if(!state.downloadItems.length){state.state='cached';return;}
          if(state.downloadItems.length>18){state.state='blocked';state.message='DEM 은 한 번에 18매까지만 신청할 수 있습니다. 범위를 좁혀 주세요.';return;}
          state.state='selected';
        }).catch(error => { if(window.__kaDemSelection===state) {
          state.state='blocked'; state.message='공개DEM 도엽 또는 포털 구조를 확인하지 못했습니다. 공식 화면에서 직접 선택해 주세요.';
        }});
    }
    return result(state.state,state.message,{items:state.items || [],downloadItems:state.downloadItems || [],
      cached:state.cached || [],unavailable:[]});
  }
  if (input.command === 'openForm') {
    const token=key+':'+input.number;
    let state=window.__kaTopographicOrder;
    if (!state || state.token!==token) {
      state=window.__kaTopographicOrder={token,state:'checkingLogin',loginStarted:Date.now(),
        nextLoginCheck:0,loginChecks:0,loginFailures:0,loginInFlight:false};
    }
    const awaitingLogin=state.state==='checkingLogin' || state.state==='login';
    if(awaitingLogin && !state.loginInFlight && Date.now()>=state.nextLoginCheck &&
        (state.loginChecks>=120 || Date.now()-state.loginStarted>=300000)) {
      state.state='blocked';state.message='로그인 확인 시간이 초과되었습니다. 계정 설정과 인증 상태를 확인한 뒤 다시 시도하세요.';
    }
    if((state.state==='checkingLogin' || state.state==='login') && !state.loginInFlight && Date.now()>=state.nextLoginCheck) {
      // 로그인 확인은 한 번에 하나만, 기한을 두고 한다. 차례마다 로그인 창을 다시 열거나 신청을 다시 내지 않는다.
      state.loginInFlight=true;++state.loginChecks;state.state='checkingLogin';
      fetch('/mn/loginCheck.do',{method:'POST',credentials:'same-origin',signal:AbortSignal.timeout(30000)})
        .then(r=>{if(!r.ok)throw new Error('http');return r.json();})
        .then(data=>{
          if(window.__kaTopographicOrder!==state)return;
          if(!data || typeof data.isLogin!=='boolean')throw new Error('loginResponse');
          state.loginFailures=0;
          if(!data.isLogin){state.state='login';state.nextLoginCheck=Date.now()+1500;return;}
          state.state='openingForm';
          const records=Array.isArray(input.records)?input.records:[input.record];
          if(!records.length || records.length>18 || records.some(r=>!r || !/^\d{5,6}$/.test(String(r.num)) ||
              !/^\d{4}$/.test(String(r.year)) || typeof r.name!=='string' || !r.name) ||
              new Set(records.map(r=>String(r.num))).size!==records.length ||
              typeof window.downloadMapData!=='function')throw new Error('selection');
          const previous=document.querySelector('.popup.mapApplication.online input[name="onBrthdy"]');
          if(visible(previous))throw new Error('openForm');
          state.previousBirth=previous;
          state.state='waitingForm';
          // 공식 화면이 「공개DEM」 다운로드 단추에서 부르는 것과 같은 호출이다.
          window.downloadMapData('4','online',records.map(r=>({types:'index',years:String(r.year),nums:String(r.num),names:r.name})),null);
        }).catch(error=>{
          if(window.__kaTopographicOrder!==state)return;
          state.errorCode=String(error && error.message).slice(0,80);
          if(state.state==='checkingLogin' && ++state.loginFailures<3){
            state.nextLoginCheck=Date.now()+1500*state.loginFailures;
            return;
          }
          state.state='blocked';state.message='공식 로그인 또는 신청 화면을 확인하지 못했습니다. 연결 상태를 확인한 뒤 다시 시도하세요.';
        }).finally(()=>{if(window.__kaTopographicOrder===state)state.loginInFlight=false;});
    }
    if(state.state==='login') {
      if(!state.loginOpened && typeof window.fnLogin==='function'){state.loginOpened=true;window.fnLogin();}
      return result('waitingLogin');
    }
    if(state.state==='waitingForm') {
      const birth=document.querySelector('.popup.mapApplication.online input[name="onBrthdy"]');
      if(birth && birth!==state.previousBirth && visible(birth))return result('form');
    }
    return result(state.state,state.message,{detail:state.errorCode});
  }
  return result('blocked','이 단계는 공개DEM 자동 진행에 아직 옮기지 못했습니다. 공식 화면에서 계속해 주세요.');
})
