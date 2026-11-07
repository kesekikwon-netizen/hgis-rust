# Antigravity Desktop CDP 브리지 (지시자 전용)
# 사용법:
#   .\scripts\ag-cdp.ps1 -Action list
#   .\scripts\ag-cdp.ps1 -Action eval  -Expr "document.title"
#   .\scripts\ag-cdp.ps1 -Action shot  -Out target\ag.png
#   .\scripts\ag-cdp.ps1 -Action send  -Text "지시 내용"
param(
  [ValidateSet('list','eval','shot','send','input-dump')]
  [string]$Action = 'list',
  [string]$Expr = 'document.title',
  [string]$Text = '',
  [string]$Out = 'target\ag.png',
  [int]$Port = 54412,
  [int]$TimeoutMs = 20000
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
  param([string]$WsUrl, [object[]]$Messages, [scriptblock]$OnEvent)
  $ws = [System.Net.WebSockets.ClientWebSocket]::new()
  $cts = [System.Threading.CancellationTokenSource]::new($TimeoutMs)
  $ws.ConnectAsync([Uri]$WsUrl, $cts.Token).GetAwaiter().GetResult()
  $buf = New-Object byte[] 65536
  $results = @()
  try {
    foreach ($m in $Messages) {
      $json = $m | ConvertTo-Json -Depth 20 -Compress
      $bytes = [System.Text.Encoding]::UTF8.GetBytes($json)
      $seg = [System.ArraySegment[byte]]::new($bytes)
      $ws.SendAsync($seg, [System.Net.WebSockets.WebSocketMessageType]::Text, $true, $cts.Token).GetAwaiter().GetResult()
    }
    while ($true) {
      $sb = New-Object System.Text.StringBuilder
      do {
        $seg = [System.ArraySegment[byte]]::new($buf)
        $r = $ws.ReceiveAsync($seg, $cts.Token).GetAwaiter().GetResult()
        if ($r.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) { break }
        [void]$sb.Append([System.Text.Encoding]::UTF8.GetString($buf, 0, $r.Count))
      } while (-not $r.EndOfMessage)
      if ($r.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) { break }
      $obj = $sb.ToString() | ConvertFrom-Json
      $results += $obj
      if ($OnEvent) { $stop = & $OnEvent $obj; if ($stop) { break } }
    }
  } finally {
    try { $ws.CloseAsync([System.Net.WebSockets.WebSocketCloseStatus]::NormalClosure, 'done', [System.Threading.CancellationToken]::None).GetAwaiter().GetResult() } catch {}
    $ws.Dispose()
  }
  return $results
}

switch ($Action) {
  'list' {
    Get-Targets | Select-Object type,title,@{n='url';e={ if($_.url.Length -gt 120){$_.url.Substring(0,120)}else{$_.url} }},id | Format-Table -AutoSize | Out-String -Width 200
  }
  'eval' {
    $t = Get-PageTarget
    $id = 1
    $msgs = @(
      @{ id = $id; method = 'Runtime.evaluate'; params = @{ expression = $Expr; returnByValue = $true; awaitPromise = $true } }
    )
    $res = Invoke-Cdp -WsUrl $t.webSocketDebuggerUrl -Messages $msgs -OnEvent { param($o) if ($o.id -eq 1) { return $true } else { return $false } }
    $r = $res | Where-Object { $_.id -eq 1 } | Select-Object -First 1
    if ($r.result.result.value -ne $null) { $r.result.result.value }
    elseif ($r.result.exceptionDetails) { 'EXCEPTION: ' + $r.result.exceptionDetails.text }
    else { ($r.result.result | ConvertTo-Json -Depth 10) }
  }
  'shot' {
    $t = Get-PageTarget
    $msgs = @(@{ id = 1; method = 'Page.captureScreenshot'; params = @{ format = 'png' } })
    $res = Invoke-Cdp -WsUrl $t.webSocketDebuggerUrl -Messages $msgs -OnEvent { param($o) if ($o.id -eq 1) { return $true } else { return $false } }
    $r = $res | Where-Object { $_.id -eq 1 } | Select-Object -First 1
    $full = Join-Path (Get-Location) $Out
    [System.IO.File]::WriteAllBytes($full, [Convert]::FromBase64String($r.result.data))
    "saved $full ($((Get-Item $full).Length) bytes)"
  }
  'input-dump' {
    $js = @'
(() => {
  const out = [];
  document.querySelectorAll('textarea, [contenteditable="true"], input[type="text"], button').forEach((el, i) => {
    const r = el.getBoundingClientRect();
    out.push({ i, tag: el.tagName, ce: el.getAttribute('contenteditable'), cls: (el.className||'').toString().slice(0,80),
               ph: el.getAttribute('placeholder') || el.getAttribute('aria-label') || '',
               txt: (el.innerText||el.value||'').slice(0,60), x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height) });
  });
  return JSON.stringify(out);
})()
'@
    $t = Get-PageTarget
    $msgs = @(@{ id = 1; method = 'Runtime.evaluate'; params = @{ expression = $js; returnByValue = $true } })
    $res = Invoke-Cdp -WsUrl $t.webSocketDebuggerUrl -Messages $msgs -OnEvent { param($o) if ($o.id -eq 1) { return $true } else { return $false } }
    $r = $res | Where-Object { $_.id -eq 1 } | Select-Object -First 1
    $r.result.result.value | ConvertFrom-Json | Format-Table -AutoSize | Out-String -Width 200
  }
  'send' {
    if (-not $Text) { throw 'send needs -Text' }
    $b64 = [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($Text))
    $js = @"
(async () => {
  const text = decodeURIComponent(escape(atob('$b64')));
  const box = document.querySelector('[contenteditable="true"]') || document.querySelector('textarea');
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
  await new Promise(r => setTimeout(r, 300));
  return 'TYPED:' + (box.innerText || box.value || '').length;
})()
"@
    $t = Get-PageTarget
    $msgs = @(@{ id = 1; method = 'Runtime.evaluate'; params = @{ expression = $js; returnByValue = $true; awaitPromise = $true } })
    $res = Invoke-Cdp -WsUrl $t.webSocketDebuggerUrl -Messages $msgs -OnEvent { param($o) if ($o.id -eq 1) { return $true } else { return $false } }
    $r = $res | Where-Object { $_.id -eq 1 } | Select-Object -First 1
    if ($r.result.exceptionDetails) { 'EXCEPTION: ' + $r.result.exceptionDetails.text } else { $r.result.result.value }
  }
}
