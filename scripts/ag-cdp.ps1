# Antigravity CDP 브리지 (지시자/작업자 공용)
# 사용법:
#   .\scripts\ag-cdp.ps1 -Action list
#   .\scripts\ag-cdp.ps1 -Action eval  -Expr "document.title"
#   .\scripts\ag-cdp.ps1 -Action eval  -ExprFile target\x.js
#   .\scripts\ag-cdp.ps1 -Action shot  -Out target\ag.png
#   .\scripts\ag-cdp.ps1 -Action send  -Text "지시 내용"
param(
  [ValidateSet('list','eval','shot','send','input-dump')]
  [string]$Action = 'list',
  [string]$Expr = 'document.title',
  [string]$ExprFile = '',
  [string]$Text = '',
  [string]$Out = 'target\ag.png',
  [int]$Port = 54412,
  [int]$TimeoutMs = 30000
)
$ErrorActionPreference = 'Stop'

function Get-Targets {
  $j = (Invoke-WebRequest "http://127.0.0.1:$Port/json/list" -UseBasicParsing -TimeoutSec 5).Content | ConvertFrom-Json
  return @($j)
}

function Get-PageTarget {
  $ts = Get-Targets
  $p = $ts | Where-Object { $_.type -eq 'page' -and $_.url -like '*antigravity*' } | Select-Object -First 1
  if (-not $p) { $p = $ts | Where-Object { $_.type -eq 'page' } | Select-Object -First 1 }
  if (-not $p) { throw 'Antigravity page target not found' }
  return $p
}

function Invoke-Cdp {
  param([string]$WsUrl, [object[]]$Messages, [int[]]$StopIds)
  $ws = [System.Net.WebSockets.ClientWebSocket]::new()
  $cts = [System.Threading.CancellationTokenSource]::new($TimeoutMs)
  $ws.ConnectAsync([Uri]$WsUrl, $cts.Token).GetAwaiter().GetResult()
  $buf = New-Object byte[] 131072
  $results = @{}
  try {
    foreach ($m in $Messages) {
      $json = $m | ConvertTo-Json -Depth 20 -Compress
      $bytes = [System.Text.Encoding]::UTF8.GetBytes($json)
      $seg = [System.ArraySegment[byte]]::new($bytes)
      $ws.SendAsync($seg, [System.Net.WebSockets.WebSocketMessageType]::Text, $true, $cts.Token).GetAwaiter().GetResult()
    }
    while ($results.Count -lt $StopIds.Count) {
      $sb = New-Object System.Text.StringBuilder
      do {
        $seg = [System.ArraySegment[byte]]::new($buf)
        $r = $ws.ReceiveAsync($seg, $cts.Token).GetAwaiter().GetResult()
        if ($r.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) { break }
        [void]$sb.Append([System.Text.Encoding]::UTF8.GetString($buf, 0, $r.Count))
      } while (-not $r.EndOfMessage)
      if ($r.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) { break }
      $obj = $sb.ToString() | ConvertFrom-Json
      if ($obj.PSObject.Properties.Name -contains 'id' -and $obj.id -ne $null) {
        if ($StopIds -contains [int]$obj.id) { $results[[int]$obj.id] = $obj }
      }
    }
  } finally {
    try { $ws.CloseAsync([System.Net.WebSockets.WebSocketCloseStatus]::NormalClosure, 'done', [System.Threading.CancellationToken]::None).GetAwaiter().GetResult() } catch {}
    $ws.Dispose()
  }
  return $results
}

function Get-EvalValue($res, [int]$id) {
  $r = $res[$id]
  if (-not $r) { return 'NO_RESULT' }
  if ($r.result.exceptionDetails) { return ('EXCEPTION: ' + ($r.result.exceptionDetails | ConvertTo-Json -Depth 5 -Compress)) }
  $v = $r.result.result.value
  if ($null -eq $v) { return 'NULL' }
  if ($v -is [string]) { return $v }
  return ($v | ConvertTo-Json -Depth 12 -Compress)
}

switch ($Action) {
  'list' {
    Get-Targets | Select-Object type,title,@{n='url';e={ if($_.url.Length -gt 120){$_.url.Substring(0,120)}else{$_.url} }},id | Format-Table -AutoSize | Out-String -Width 200
  }
  'eval' {
    if ($ExprFile) { $Expr = [System.IO.File]::ReadAllText((Join-Path (Get-Location) $ExprFile)) }
    $t = Get-PageTarget
    $msgs = @(@{ id = 1; method = 'Runtime.evaluate'; params = @{ expression = $Expr; returnByValue = $true; awaitPromise = $true } })
    $res = Invoke-Cdp -WsUrl $t.webSocketDebuggerUrl -Messages $msgs -StopIds @(1)
    Get-EvalValue $res 1
  }
  'shot' {
    $t = Get-PageTarget
    $msgs = @(@{ id = 1; method = 'Page.captureScreenshot'; params = @{ format = 'png' } })
    $res = Invoke-Cdp -WsUrl $t.webSocketDebuggerUrl -Messages $msgs -StopIds @(1)
    $full = Join-Path (Get-Location) $Out
    [System.IO.File]::WriteAllBytes($full, [Convert]::FromBase64String($res[1].result.data))
    "saved $full ($((Get-Item $full).Length) bytes)"
  }
  'input-dump' {
    if (-not $ExprFile) {
      $Expr = @'
(() => {
  const out = [];
  document.querySelectorAll('div[contenteditable="true"], textarea').forEach((el, i) => {
    const r = el.getBoundingClientRect();
    out.push({ i, tag: el.tagName, len: (el.innerText || el.value || '').length,
               head: (el.innerText || el.value || '').slice(0, 80),
               x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height) });
  });
  const btns = [...document.querySelectorAll('button')]
    .filter(b => (b.getAttribute('aria-label') || '').toLowerCase().includes('send'))
    .map(b => ({ label: b.getAttribute('aria-label'), disabled: b.disabled }));
  return JSON.stringify({ inputs: out, sendButtons: btns });
})()
'@
    } else { $Expr = [System.IO.File]::ReadAllText((Join-Path (Get-Location) $ExprFile)) }
    $t = Get-PageTarget
    $msgs = @(@{ id = 1; method = 'Runtime.evaluate'; params = @{ expression = $Expr; returnByValue = $true } })
    $res = Invoke-Cdp -WsUrl $t.webSocketDebuggerUrl -Messages $msgs -StopIds @(1)
    Get-EvalValue $res 1
  }
  'send' {
    if (-not $Text) { throw 'send needs -Text' }
    $t = Get-PageTarget
    $ws = $t.webSocketDebuggerUrl
    $b64 = [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($Text))

    # 1) 입력창에 텍스트 삽입 (execCommand 경로: React/DOM 양쪽에서 동작 확인됨)
    $fillJs = @"
(() => {
  const text = decodeURIComponent(escape(atob('$b64')));
  const box = document.querySelector('div[contenteditable="true"]') || document.querySelector('textarea');
  if (!box) return 'NO_INPUT';
  box.focus();
  if (box.tagName === 'TEXTAREA') {
    const setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set;
    setter.call(box, text);
    box.dispatchEvent(new Event('input', { bubbles: true }));
  } else {
    document.execCommand('selectAll', false, null);
    document.execCommand('insertText', false, text);
  }
  const len = (box.innerText || box.value || '').length;
  return 'FILLED:' + len;
})()
"@
    $res = Invoke-Cdp -WsUrl $ws -Messages @(@{ id = 1; method = 'Runtime.evaluate'; params = @{ expression = $fillJs; returnByValue = $true } }) -StopIds @(1)
    $filled = Get-EvalValue $res 1
    if ($filled -like 'NO_INPUT*') { return $filled }

    # 2) 신뢰할 수 있는 Enter 키 (CDP Input.dispatchKeyEvent — 합성 KeyboardEvent 는 무시됨)
    $enter = @(
      @{ id = 2; method = 'Input.dispatchKeyEvent'; params = @{ type='keyDown'; key='Enter'; code='Enter'; windowsVirtualKeyCode=13; nativeVirtualKeyCode=13; text="`r" } },
      @{ id = 3; method = 'Input.dispatchKeyEvent'; params = @{ type='keyUp';   key='Enter'; code='Enter'; windowsVirtualKeyCode=13; nativeVirtualKeyCode=13 } }
    )
    $res2 = Invoke-Cdp -WsUrl $ws -Messages $enter -StopIds @(2,3)
    Start-Sleep -Milliseconds 700

    # 3) 입력창이 비었는지 확인 (비었으면 전송 성공)
    $checkJs = @'
(() => {
  const box = document.querySelector('div[contenteditable="true"]') || document.querySelector('textarea');
  if (!box) return JSON.stringify({ gone: true, left: 0 });
  return JSON.stringify({ gone: false, left: (box.innerText || box.value || '').trim().length });
})()
'@
    $res3 = Invoke-Cdp -WsUrl $ws -Messages @(@{ id = 4; method = 'Runtime.evaluate'; params = @{ expression = $checkJs; returnByValue = $true } }) -StopIds @(4)
    $chk = (Get-EvalValue $res3 4) | ConvertFrom-Json

    if ($chk.gone -or $chk.left -eq 0) { return "SENT_ENTER ($filled)" }

    # 4) Enter 가 안 먹었으면 전송 버튼 클릭
    $clickJs = @'
(() => {
  const btn = [...document.querySelectorAll('button')].find(b => {
    const l = ((b.getAttribute('aria-label') || '') + ' ' + (b.getAttribute('title') || '')).toLowerCase();
    return l.includes('send message') || l.includes('send');
  });
  if (!btn) return 'NO_SEND_BTN';
  if (btn.disabled) return 'SEND_DISABLED';
  btn.click();
  return 'CLICKED';
})()
'@
    $res4 = Invoke-Cdp -WsUrl $ws -Messages @(@{ id = 5; method = 'Runtime.evaluate'; params = @{ expression = $clickJs; returnByValue = $true } }) -StopIds @(5)
    $clicked = Get-EvalValue $res4 5
    Start-Sleep -Milliseconds 900

    $res5 = Invoke-Cdp -WsUrl $ws -Messages @(@{ id = 6; method = 'Runtime.evaluate'; params = @{ expression = $checkJs; returnByValue = $true } }) -StopIds @(6)
    $chk2 = (Get-EvalValue $res5 6) | ConvertFrom-Json
    if ($chk2.gone -or $chk2.left -eq 0) { return "SENT_BUTTON ($clicked, $filled)" }
    return "SEND_FAILED left=$($chk2.left) click=$clicked fill=$filled"
  }
}
