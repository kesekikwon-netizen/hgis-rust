# Hidden managed printer boundary. Strata C++: src/app/KaPrintDialog.cpp:83-124,620-669.
# No policy changes, dialogs, shells or user-supplied executable statements.
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
Add-Type -AssemblyName System.Drawing

function Get-PrintableMm($page, [bool]$landscape) {
    $page.Landscape = $landscape
    $r = $page.PrintableArea
    # PrintableArea swaps device axes for Landscape; undo that representation for physical page mm.
    # PageSettings.cs:134-159 (dotnet/winforms); confirmed against this PC's System.Drawing4.0 Bounds.
    if ($landscape) { return @(($r.Y * 0.254), ($r.X * 0.254), ($r.Height * 0.254), ($r.Width * 0.254)) }
    return @(($r.X * 0.254), ($r.Y * 0.254), ($r.Width * 0.254), ($r.Height * 0.254))
}

function Paint-Part($g, $image, $source, $target, $drawing) {
    $w = [Math]::Min($source[2], $drawing[0] - $source[0])
    $h = [Math]::Min($source[3], $drawing[1] - $source[1])
    if ($w -le 0 -or $h -le 0) { return }
    $dest = [System.Drawing.RectangleF]::new($target[0], $target[1], ($target[2] * $w / $source[2]), ($target[3] * $h / $source[3]))
    $crop = [System.Drawing.RectangleF]::new(($source[0] * $image.Width / $drawing[0]), ($source[1] * $image.Height / $drawing[1]),
        ($w * $image.Width / $drawing[0]), ($h * $image.Height / $drawing[1]))
    $g.DrawImage($image, $dest, $crop, [System.Drawing.GraphicsUnit]::Pixel)
}

function Paint-Page($e, [int]$index) {
    $request = $script:request
    $plan = $request.plan
    $actual = @(($e.PageBounds.Width * 0.254), ($e.PageBounds.Height * 0.254))
    if ([Math]::Abs($actual[0] - $plan.paper_mm[0]) -gt 2 -or [Math]::Abs($actual[1] - $plan.paper_mm[1]) -gt 2) {
        throw 'Printer kept a different physical page size'
    }
    $script:actualPaper = $actual
    $g = $e.Graphics
    $g.PageUnit = [System.Drawing.GraphicsUnit]::Millimeter
    # OriginAtMargins=false starts at the printable corner. Return to physical paper coordinates.
    # https://learn.microsoft.com/dotnet/api/system.drawing.printing.printdocument.originatmargins
    $g.TranslateTransform(-($e.PageSettings.HardMarginX * 0.254), -($e.PageSettings.HardMarginY * 0.254))
    $box = $plan.printable_mm
    $g.SetClip([System.Drawing.RectangleF]::new($box[0], $box[1], $box[2], $box[3]))
    $g.FillRectangle([System.Drawing.Brushes]::White, [System.Drawing.RectangleF]::new(0, 0, $plan.paper_mm[0], $plan.paper_mm[1]))
    $pen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(90, 88, 82), 0.2)
    $font = [System.Drawing.Font]::new('Malgun Gothic', 8)
    try {
        if ($plan.overview -and $index -eq 0) {
            $extentW = [double]$request.drawing_mm[0]
            $extentH = [double]$request.drawing_mm[1]
            foreach ($part in $plan.pages) {
                $extentW = [Math]::Max($extentW, $part.source_mm[0] + $part.source_mm[2])
                $extentH = [Math]::Max($extentH, $part.source_mm[1] + $part.source_mm[3])
            }
            $scale = [Math]::Min($box[2] / $extentW, ($box[3] - 30) / $extentH)
            $originX = $box[0] + ($box[2] - $extentW * $scale) / 2
            $originY = $box[1] + 8
            $target = @($originX, $originY, ($request.drawing_mm[0] * $scale), ($request.drawing_mm[1] * $scale))
            Paint-Part $g $script:picture @(0, 0, $request.drawing_mm[0], $request.drawing_mm[1]) $target $request.drawing_mm
            $number = 1
            foreach ($part in $plan.pages) {
                $r = $part.source_mm
                $rect = [System.Drawing.RectangleF]::new(($originX + $r[0] * $scale), ($originY + $r[1] * $scale), ($r[2] * $scale), ($r[3] * $scale))
                $g.DrawRectangle($pen, $rect.X, $rect.Y, $rect.Width, $rect.Height)
                $g.DrawString([string]$number, $font, [System.Drawing.Brushes]::Black, $rect.X + 1, $rect.Y + 1)
                $number++
            }
            $g.DrawString($request.title, $font, [System.Drawing.Brushes]::Black, [single]$box[0], [single]$box[1])
            $g.DrawString('1. 번호 순서대로 늘어놓습니다. 1번이 왼쪽 위입니다.', $font, [System.Drawing.Brushes]::Black, [single]$box[0], [single]($box[1] + $box[3] - 16))
            $g.DrawString('4. 각 장 아래의 확인선을 자로 재어 5 cm면 크기가 맞게 찍힌 것입니다.', $font, [System.Drawing.Brushes]::Black, [single]$box[0], [single]($box[1] + $box[3] - 9))
        } else {
            $offset = 0
            if ($plan.overview) { $offset = 1 }
            $part = $plan.pages[$index - $offset]
            if ($script:pageImages.Count -gt 0) {
                $tile = [System.Drawing.Image]::FromFile($script:pageImages[$index - $offset])
                try { Paint-Part $g $tile @(0, 0, $part.source_mm[2], $part.source_mm[3]) $part.target_mm @($part.source_mm[2], $part.source_mm[3]) }
                finally { $tile.Dispose() }
            } else { Paint-Part $g $script:picture $part.source_mm $part.target_mm $request.drawing_mm }
            if ($plan.marks) {
                $r = $part.target_mm
                $pen.DashStyle = [System.Drawing.Drawing2D.DashStyle]::Dash
                if ($part.col + 1 -lt $plan.cols) { $g.DrawLine($pen, [single]($r[0] + $r[2] - $plan.overlap_mm), [single]$r[1], [single]($r[0] + $r[2] - $plan.overlap_mm), [single]($r[1] + $r[3])) }
                if ($part.row + 1 -lt $plan.rows) { $g.DrawLine($pen, [single]$r[0], [single]($r[1] + $r[3] - $plan.overlap_mm), [single]($r[0] + $r[2]), [single]($r[1] + $r[3] - $plan.overlap_mm)) }
                $pen.DashStyle = [System.Drawing.Drawing2D.DashStyle]::Solid
                $y = [single]($r[1] + $r[3] + 3)
                $g.DrawLine($pen, [single]$r[0], $y, [single]($r[0] + 50), $y)
                $g.DrawString(('5 cm · ' + ($index - $offset + 1) + '/' + $plan.pages.Count), $font, [System.Drawing.Brushes]::Black, [single]($r[0] + 55), [single]($y - 2))
            }
        }
    } finally { $pen.Dispose(); $font.Dispose() }
    $script:pagesPainted++
}

try {
    if ($env:STRATA_PRINT_MODE -eq 'Catalog') {
        $ports = @{}
        Get-CimInstance -ClassName Win32_Printer | ForEach-Object { $ports[$_.Name] = $_.PortName }
        $printers = @([System.Drawing.Printing.PrinterSettings]::InstalledPrinters | ForEach-Object {
            $settings = [System.Drawing.Printing.PrinterSettings]::new()
            $settings.PrinterName = $_
            $name = $_
            if ($settings.IsValid) {
                $papers = @($settings.PaperSizes | Where-Object {
                    $w = [Math]::Min($_.Width, $_.Height) * 0.254
                    $h = [Math]::Max($_.Width, $_.Height) * 0.254
                    ([Math]::Abs($w - 210) -le 2 -and [Math]::Abs($h - 297) -le 2) -or
                    ([Math]::Abs($w - 297) -le 2 -and [Math]::Abs($h - 420) -le 2)
                } | ForEach-Object {
                    $page = [System.Drawing.Printing.PageSettings]::new($settings)
                    $page.PaperSize = $_
                    @{
                        name = $_.PaperName
                        width_mm = [Math]::Min($_.Width, $_.Height) * 0.254
                        height_mm = [Math]::Max($_.Width, $_.Height) * 0.254
                        portrait = @(Get-PrintableMm $page $false)
                        landscape = @(Get-PrintableMm $page $true)
                    }
                })
                @{
                    name = $name
                    is_default = $settings.IsDefaultPrinter
                    file_output = $ports[$name] -in @('PORTPROMPT:', 'FILE:')
                    papers = $papers
                }
            }
        })
        @{ version = 1; printers = $printers } | ConvertTo-Json -Depth 6 -Compress
        exit 0
    }
    if ($env:STRATA_PRINT_MODE -notin @('Print', 'Probe')) { throw 'Unknown printer operation' }
    $payload = Get-Content -LiteralPath $env:STRATA_PRINT_REQUEST -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($payload.version -ne 1) { throw 'Unknown printer request version' }
    $script:request = $payload.request
    $script:pageImages = @()
    if ($null -ne $payload.page_images) { $script:pageImages = @($payload.page_images) }
    if ($script:pageImages.Count -ne 0 -and $script:pageImages.Count -ne $script:request.plan.pages.Count) { throw 'Missing print page image' }
    $script:picture = [System.Drawing.Image]::FromFile($payload.image)
    $script:pagesPainted = 0
    $script:actualPaper = @()
    $total = $script:request.plan.pages.Count
    if ($script:request.plan.overview) { $total++ }
    try {
        if ($env:STRATA_PRINT_MODE -eq 'Probe') {
            $folder = [IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($env:STRATA_PRINT_REQUEST))
            for ($index = 0; $index -lt $total; $index++) {
                $mm = $script:request.plan.paper_mm
                $bitmap = [System.Drawing.Bitmap]::new([int][Math]::Ceiling($mm[0] * 4), [int][Math]::Ceiling($mm[1] * 4))
                $bitmap.SetResolution(101.6, 101.6)
                $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
                try {
                    $graphics.Clear([System.Drawing.Color]::White)
                    $event = [pscustomobject]@{ Graphics = $graphics; PageBounds = [System.Drawing.Rectangle]::new(0, 0, [int][Math]::Round($mm[0] / 0.254), [int][Math]::Round($mm[1] / 0.254)); PageSettings = [pscustomobject]@{ HardMarginX = 0; HardMarginY = 0 } }
                    Paint-Page $event $index
                    $bitmap.Save((Join-Path $folder ('page-{0:D4}.png' -f ($index + 1))), [System.Drawing.Imaging.ImageFormat]::Png)
                } finally { $graphics.Dispose(); $bitmap.Dispose() }
            }
        } else {
            $document = [System.Drawing.Printing.PrintDocument]::new()
            try {
                $document.PrinterSettings.PrinterName = $script:request.printer
                if (-not $document.PrinterSettings.IsValid) { throw 'Printer is no longer available' }
                $requested = $script:request.paper
                $paper = @($document.PrinterSettings.PaperSizes | Where-Object { $_.PaperName -eq $requested.name }) | Select-Object -First 1
                if ($null -eq $paper) { throw 'Printer no longer supports this paper' }
                $document.DefaultPageSettings.PaperSize = $paper
                $document.DefaultPageSettings.Landscape = $script:request.plan.landscape
                $document.PrinterSettings.Duplex = [System.Drawing.Printing.Duplex]::Simplex
                if ($script:request.output) {
                    $document.PrinterSettings.PrintToFile = $true
                    $document.PrinterSettings.PrintFileName = $script:request.output
                }
                $document.DocumentName = $script:request.title
                $document.OriginAtMargins = $false
                $document.PrintController = [System.Drawing.Printing.StandardPrintController]::new()
                $script:pageIndex = 0
                $script:printError = $null
                $document.add_PrintPage({
                    param($sender, $event)
                    try {
                        Paint-Page $event $script:pageIndex
                        $script:pageIndex++
                        $event.HasMorePages = $script:pageIndex -lt $total
                    } catch {
                        $script:printError = $_.Exception.Message
                        $event.Cancel = $true
                        $event.HasMorePages = $false
                    }
                })
                $document.Print()
                if ($script:printError) { throw $script:printError }
            } finally { $document.Dispose() }
        }
        if ($script:pagesPainted -ne $total) { throw 'Printer did not accept every page' }
        @{ version = 1; printer = $script:request.printer; pages = $script:pagesPainted; paper_mm = $script:actualPaper } | ConvertTo-Json -Compress
    } finally { $script:picture.Dispose() }
} catch {
    [Console]::Error.WriteLine($_.Exception.Message)
    exit 1
}
