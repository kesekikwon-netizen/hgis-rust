// 국토정보플랫폼 「공개DEM」 — 수치지형도와 같은 통로(로그인 → 신청 → 내려받기)에서 DEM 을 고른다.
// 공개 요청은 2026-11-11 실제 화면에서 확인했다: demQuery.do, downloadMapData('4','online',…).
// 계정·토큰·파일 주소는 다루지 않는다. 믿는 포털 화면 안에서만 돈다. 결과는 JSON 으로 돌려준다.
// 이 파일은 select·openForm·form·submit·files·download 를 맡는다(로그인·신청 확인·전송 화면 설정은 ngii-topographic.js).
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
  if (input.command === 'form' || input.command === 'submit') {
    const changed = e => { e.dispatchEvent(new Event('input', {bubbles:true})); e.dispatchEvent(new Event('change', {bubbles:true})); };
    const forms=Array.from(document.querySelectorAll('.popup.mapApplication.online')).filter(visible);
    if(forms.length>1)return result('blocked','신청서가 여러 개 열려 있습니다. 공식 화면에서 현재 신청서를 확인해 주세요.');
    const form=forms[0];
    if(!visible(form))return result('waitingForm');
    const birth=form.querySelector('input[name="onBrthdy"]'), purpose=form.querySelector('#onPurchsPurps');
    const group=form.querySelector('select[name="code"]'), basis=form.querySelector('input[name="onBasis"]');
    if(!birth || !purpose || purpose.tagName!=='SELECT' || !basis)
      return result('blocked','신청서 구조가 달라 자동 입력을 중단했습니다. 포털에서 신청서를 확인해 주세요.');
    if(!birth.value.trim() && input.birthDate){birth.value=input.birthDate;changed(birth);}
    // 사용목적은 공식 목록에서 고른다(2026-11-11 확인: 「제작 및 활용」 아래 CAAAC00001~7). 없는 값은 넣지 않는다.
    const code=String(input.purposeCode || '');
    if(!purpose.value && code) {
      if(group && !group.value && Array.from(group.options).some(o=>o.value===code.slice(0,5))){group.value=code.slice(0,5);changed(group);}
      if(Array.from(purpose.options).some(o=>o.value===code)){purpose.value=code;changed(purpose);}
    }
    if(!basis.value.trim() && input.purpose){basis.value=input.purpose;changed(basis);}
    if(!birth.value.trim())return result('input','신청에 필요한 생년월일이 비어 있습니다. 아래 입력란 또는 공식 신청서에 입력해 주세요.');
    if(!purpose.value)return result(code?'waitingForm':'input','사용목적이 비어 있습니다. 공식 신청서에서 골라 주세요.');
    if(!basis.value.trim())return result('input','상세용도가 비어 있습니다. 아래 입력란 또는 공식 신청서에 입력해 주세요.');
    const agree=form.querySelector('input[name="onChkAgree"]#agree');
    const submit=Array.from(form.querySelectorAll('button,input[type=button],input[type=submit],a'))
      .filter(e => visible(e) && (e.textContent || e.value || '').trim() === '다운로드');
    if(!agree || !['radio','checkbox'].includes(agree.type) || submit.length!==1 || typeof window.checkValidation!=='function')
      return result('blocked','준수사항 또는 다운로드 버튼을 정확히 확인하지 못했습니다. 공식 신청서에서 계속해 주세요.');
    if(input.command==='form')return result('ready');
    agree.checked=true;changed(agree);
    if(!window.checkValidation('4','online'))return result('input','공식 신청서의 필수 항목을 확인해 주세요.');
    const events=window.jQuery && window.jQuery(document);
    if(!events || typeof events.on!=='function' || typeof events.off!=='function')
      return result('blocked','공식 신청 결과를 확인할 수 없어 자동 제출을 중단했습니다.');
    // 수치지형도 스크립트의 order 명령이 같은 기록을 읽는다.
    const submission=window.__kaTopographicSubmission={key,number:input.number,state:'pending',started:Date.now()};
    events.off('ajaxComplete.kaTopographicOrder');
    events.on('ajaxComplete.kaTopographicOrder',function(event,xhr,settings){
      if(window.__kaTopographicSubmission!==submission)return;
      let url;try{url=new URL(settings.url,location.href);}catch(e){return;}
      if(url.origin!==location.origin || url.pathname!=='/pd/shbtManage/shbtInsertTritVidoDem.do')return;
      // 결과 표시만 본다. 요청 내용이나 개인정보는 남기지 않는다(응답은 항목 이름만).
      let data=xhr.responseJSON;
      if(!data){try{data=JSON.parse(xhr.responseText);}catch(e){submission.state='failed';return;}}
      submission.shape=Object.keys(data||{}).concat(Object.keys((data&&data.allDataMap)||{}).map(k=>'allDataMap.'+k)).slice(0,40);
      if(xhr.status<200 || xhr.status>=300 || !data || data.resultCode!=='1' || !data.allDataMap){submission.state='failed';return;}
      submission.state=data.allDataMap.orderDownList ? 'ready' : 'delayed';
    });
    submit[0].click();
    return result('submitted');
  }
  if (input.command === 'files' || input.command === 'download') {
    // 공식 전송 화면(설치 없이 받는 방식으로 열린 것)에서만 한다.
    if(document.readyState!=='complete')return result('waitingFiles');
    if(location.origin!=='https://nlippd.ngii.go.kr' || location.pathname!=='/nlippd/pd/innorix/innorixdownLoad.do' ||
        !document.getElementById('fileControl'))return result('waitingFiles');
    const control=window.control;
    if(!control || typeof control.getDownloadFiles!=='function' ||
        typeof control.getOption!=='function' || typeof control.download!=='function')return result('waitingFiles');
    const options=control.getOption();
    if(!options || options.agent!==false)return result('blocked','공식 브라우저 전송을 준비하지 못했습니다. 다시 받기를 눌러 주세요.');
    const files=control.getDownloadFiles();
    if(!Array.isArray(files) || !files.length)return result('waitingFiles');
    const records=Array.isArray(input.records)?input.records:[{num:input.number}];
    const numbers=records.map(r=>String(r.num));
    const names=new Set(),ids=new Set(),manifest=[];let total=0;
    for(const file of files) {
      const name=file.printFileName,bytes=Number(file.fileSize);
      let url;try{url=new URL(file.downloadUrl,location.href);}catch(e){return result('blocked','공식 파일 주소가 올바르지 않습니다.');}
      // 도엽이 하나면 그 도엽의 파일로 본다. 여럿이면 파일 이름에 도엽 번호가 정확히 하나 들어 있어야 한다.
      const matched=numbers.length===1?numbers:(typeof name==='string'?numbers.filter(n=>new RegExp('(^|[^0-9])'+n+'([^0-9]|$)').test(name)):[]);
      if(matched.length!==1 || typeof file.id!=='string' || !file.id || file.id.length>256 || ids.has(file.id) ||
          typeof name!=='string' || !/^[^\\\/:*?"<>|\x00-\x1f]{1,200}$/.test(name) || names.has(name) ||
          !Number.isSafeInteger(bytes) || bytes<=0 || url.origin!==location.origin)
        return result('blocked','신청 도엽과 공식 파일 목록이 일치하지 않아 전송을 중단했습니다.',
          {detail:'file:'+String(name).slice(0,40)+':'+url.origin+url.pathname});
      names.add(name);ids.add(file.id);total+=bytes;
      manifest.push({id:file.id,name,number:matched[0],bytes,path:url.pathname});
    }
    if(total>1000000000)return result('blocked','선택한 파일 크기가 1GB를 넘습니다.');
    const snapshot=JSON.stringify(files.map(f=>[f.id,f.printFileName,f.fileSize,f.downloadUrl]));
    if(input.command==='files'){
      window.__kaSdkManifest={key,snapshot};
      return result('ready','',{totalBytes:total,files:files.length,serial:true,manifest});
    }
    if(!window.__kaSdkManifest || window.__kaSdkManifest.key!==key || window.__kaSdkManifest.snapshot!==snapshot)
      return result('blocked','공식 파일 목록이 변경되어 전송을 중단했습니다.');
    const index=input.fileIndex===undefined?0:input.fileIndex;
    if(!Number.isInteger(index) || index<0 || index>=files.length)return result('blocked','다음 파일 순서를 확인하지 못했습니다.');
    const token=key+':'+index;
    if(window.__kaSdkDownload===token)return result('downloading');
    window.__kaSdkDownload=token;
    if(control.download(files[index].id)===false)return result('blocked','공식 파일 전송을 시작하지 못했습니다.');
    return result('downloading');
  }
  return result('blocked','지원되지 않는 자동화 단계입니다.');
})
