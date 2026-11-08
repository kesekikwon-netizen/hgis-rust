# Minimal CDP helper: one connection, one call, reliable result.
param(
  [ValidateSet('eval','key','click','status','send')]
  [string]$Mode = 'status',
  [string]$Js = '',
  [string]$JsFile = '',
  [string]$Text = '',
  [int]$Port = 54412
)
$ErrorActionPreference = 'Stop'

function Get-Ws {
  $j = (Invoke-WebRequest "http://127.0.0.1:$Port/json/list" -UseBasicParsing -TimeoutSec 5).Content | ConvertFrom-Json
  $all = @($j)
  # the app page is a 'page' whose title mentions Antigravity; url may be data:/file:
  $t = $all | Where-Object { $_.type -eq 'page' -and $_.title -like '*Antigravity*' } | Select-Object -First 1
  if (-not $t) { $t = $all | Where-Object { $_.type -eq 'page' -and $_.url -like '*antigravity*' } | Select-Object -First 1 }
  if (-not $t) { $t = $all | Where-Object { $_.type -eq 'page' } | Select-Object -First 1 }
  if (-not $t) { throw 'no antigravity page target' }
  return $t.webSocketDebuggerUrl
}

function Call-Cdp {
  param([string]$WsUrl, [string]$Method, [hashtable]$Params, [int]$Id = 1)
  $ws = [System.Net.WebSockets.ClientWebSocket]::new()
  $cts = [System.Threading.CancellationTokenSource]::new(25000)
  try {
    $ws.ConnectAsync([Uri]$WsUrl, $cts.Token).GetAwaiter().GetResult()
    $body = @{ id = $Id; method = $Method; params = $Params } | ConvertTo-Json -Depth 20 -Compress
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($body)
    $ws.SendAsync([System.ArraySegment[byte]]::new($bytes), [System.Net.WebSockets.WebSocketMessageType]::Text, $true, $cts.Token).GetAwaiter().GetResult()
    $buf = New-Object byte[] 262144
    while ($true) {
      $sb = New-Object System.Text.StringBuilder
      do {
        $r = $ws.ReceiveAsync([System.ArraySegment[byte]]::new($buf), $cts.Token).GetAwaiter().GetResult()
        if ($r.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) { throw 'ws closed' }
        [void]$sb.Append([System.Text.Encoding]::UTF8.GetString($buf, 0, $r.Count))
      } while (-not $r.EndOfMessage)
      $o = $sb.ToString() | ConvertFrom-Json
      if ($o.PSObject.Properties.Name -contains 'id' -and [int]$o.id -eq $Id) { return $o }
    }
  } finally {
    try { $ws.CloseAsync([System.Net.WebSockets.WebSocketCloseStatus]::NormalClosure, 'x', [System.Threading.CancellationToken]::None).GetAwaiter().GetResult() } catch {}
    $ws.Dispose(); $cts.Dispose()
  }
}

$ws = Get-Ws

switch ($Mode) {
  'status' {
    $js = @'
(() => {
  const box = document.querySelector('div[contenteditable="true"]');
  const btn = [...document.querySelectorAll('button')].find(b => (b.getAttribute('aria-label')||'') === 'Send message');
  return JSON.stringify({
    inputLen: box ? (box.innerText||'').length : -1,
    inputHead: box ? (box.innerText||'').slice(0,100) : null,
    sendBtn: !!btn,
    sendDisabled: btn ? btn.disabled : null
  });
})()
'@
    $r = Call-Cdp -WsUrl $ws -Method 'Runtime.evaluate' -Params @{ expression = $js; returnByValue = $true }
    return $r.result.result.value
  }
  'eval' {
    if ($JsFile) { $Js = [System.IO.File]::ReadAllText((Join-Path (Get-Location) $JsFile)) }
    $r = Call-Cdp -WsUrl $ws -Method 'Runtime.evaluate' -Params @{ expression = $Js; returnByValue = $true; awaitPromise = $true }
    if ($r.result.exceptionDetails) { return 'EXC: ' + ($r.result.exceptionDetails | ConvertTo-Json -Depth 4 -Compress) }
    return $r.result.result.value
  }
  'key' {
    $d = Call-Cdp -WsUrl $ws -Method 'Input.dispatchKeyEvent' -Params @{ type='keyDown'; key='Enter'; code='Enter'; windowsVirtualKeyCode=13; nativeVirtualKeyCode=13; text="`r"; unmodifiedText="`r" } -Id 1
    Start-Sleep -Milliseconds 120
    $u = Call-Cdp -WsUrl $ws -Method 'Input.dispatchKeyEvent' -Params @{ type='keyUp'; key='Enter'; code='Enter'; windowsVirtualKeyCode=13; nativeVirtualKeyCode=13 } -Id 2
    return 'KEY_SENT'
  }
  'click' {
    $js = @'
(() => {
  const btn = [...document.querySelectorAll('button')].find(b => (b.getAttribute('aria-label')||'') === 'Send message');
  if (!btn) return 'NO_BTN';
  if (btn.disabled) return 'DISABLED';
  btn.click();
  return 'CLICKED';
})()
'@
    $r = Call-Cdp -WsUrl $ws -Method 'Runtime.evaluate' -Params @{ expression = $js; returnByValue = $true }
    return $r.result.result.value
  }
  'send' {
    if (-not $Text) { throw 'send needs -Text' }

    # 1) focus the composer
    $focusJs = @'
(() => {
  const box = document.querySelector('div[contenteditable="true"]');
  if (!box) return 'NO_INPUT';
  box.focus();
  const sel = window.getSelection();
  sel.removeAllRanges();
  const r = document.createRange();
  r.selectNodeContents(box);
  sel.addRange(r);
  return 'FOCUSED';
})()
'@
    $f = Call-Cdp -WsUrl $ws -Method 'Runtime.evaluate' -Params @{ expression = $focusJs; returnByValue = $true }
    if ($f.result.result.value -ne 'FOCUSED') { return "SEND_FAIL: focus=$($f.result.result.value)" }

    # 2) replace any existing text (select-all then insert via CDP)
    $null = Call-Cdp -WsUrl $ws -Method 'Input.dispatchKeyEvent' -Params @{ type='keyDown'; key='a'; code='KeyA'; windowsVirtualKeyCode=65; nativeVirtualKeyCode=65; modifiers=2 } -Id 10
    $null = Call-Cdp -WsUrl $ws -Method 'Input.dispatchKeyEvent' -Params @{ type='keyUp';   key='a'; code='KeyA'; windowsVirtualKeyCode=65; nativeVirtualKeyCode=65; modifiers=2 } -Id 11
    $null = Call-Cdp -WsUrl $ws -Method 'Input.dispatchKeyEvent' -Params @{ type='keyDown'; key='Delete'; code='Delete'; windowsVirtualKeyCode=46; nativeVirtualKeyCode=46 } -Id 12
    $null = Call-Cdp -WsUrl $ws -Method 'Input.dispatchKeyEvent' -Params @{ type='keyUp';   key='Delete'; code='Delete'; windowsVirtualKeyCode=46; nativeVirtualKeyCode=46 } -Id 13

    # 3) Input.insertText — real text insertion, works with role=combobox editors
    $null = Call-Cdp -WsUrl $ws -Method 'Input.insertText' -Params @{ text = $Text } -Id 20
    Start-Sleep -Milliseconds 250
    # 한 번에 안 들어가는 경우가 있어 입력 길이를 보고 부족분을 다시 넣는다
    for ($try = 0; $try -lt 3; $try++) {
      $probe = Call-Cdp -WsUrl $ws -Method 'Runtime.evaluate' -Params @{ expression = '(document.querySelector("div[contenteditable=true]")?.innerText||"").length'; returnByValue = $true } -Id (40 + $try)
      $cur = [int]$probe.result.result.value
      if ($cur -ge ($Text.Length - 4)) { break }
      $rest = $Text.Substring([Math]::Min($cur, $Text.Length))
      $null = Call-Cdp -WsUrl $ws -Method 'Input.insertText' -Params @{ text = $rest } -Id (50 + $try)
      Start-Sleep -Milliseconds 250
    }
    Start-Sleep -Milliseconds 400

    # 4) confirm the text landed
    $lenJs = @'
(() => {
  const box = document.querySelector('div[contenteditable="true"]');
  return box ? String((box.innerText||'').length) : '-1';
})()
'@
    $lr = Call-Cdp -WsUrl $ws -Method 'Runtime.evaluate' -Params @{ expression = $lenJs; returnByValue = $true }
    $typedLen = [int]$lr.result.result.value
    if ($typedLen -le 0) { return "SEND_FAIL: insertText produced 0 chars" }

    # 5) real Enter key (CDP, not a synthetic KeyboardEvent)
    $null = Call-Cdp -WsUrl $ws -Method 'Input.dispatchKeyEvent' -Params @{ type='rawKeyDown'; key='Enter'; code='Enter'; windowsVirtualKeyCode=13; nativeVirtualKeyCode=13 } -Id 30
    Start-Sleep -Milliseconds 60
    $null = Call-Cdp -WsUrl $ws -Method 'Input.dispatchKeyEvent' -Params @{ type='char'; key='Enter'; code='Enter'; windowsVirtualKeyCode=13; nativeVirtualKeyCode=13; text="`r"; unmodifiedText="`r" } -Id 31
    Start-Sleep -Milliseconds 60
    $null = Call-Cdp -WsUrl $ws -Method 'Input.dispatchKeyEvent' -Params @{ type='keyUp'; key='Enter'; code='Enter'; windowsVirtualKeyCode=13; nativeVirtualKeyCode=13 } -Id 32
    Start-Sleep -Milliseconds 1200

    # 6) verified result: input cleared?
    $after = Call-Cdp -WsUrl $ws -Method 'Runtime.evaluate' -Params @{ expression = $lenJs; returnByValue = $true }
    $leftLen = [int]$after.result.result.value
    if ($leftLen -gt 0) {
      # Enter did not submit -> try the Send button
      $c = Call-Cdp -WsUrl $ws -Method 'Runtime.evaluate' -Params @{ expression = @'
(() => {
  const b = [...document.querySelectorAll('button')].find(x => (x.getAttribute('aria-label')||'') === 'Send message');
  if (!b || b.disabled) return 'NO_BTN';
  b.click(); return 'CLICKED';
})()
'@; returnByValue = $true }
      Start-Sleep -Milliseconds 1400
      $after2 = Call-Cdp -WsUrl $ws -Method 'Runtime.evaluate' -Params @{ expression = $lenJs; returnByValue = $true }
      $leftLen = [int]$after2.result.result.value
      return "ENTER_FAILED_BUTTON_$($c.result.result.value): typed=$typedLen left=$leftLen"
    }
    return "SENT_ENTER_OK: typed=$typedLen left=0"
  }
}
