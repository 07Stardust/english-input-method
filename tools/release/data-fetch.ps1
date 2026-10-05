# 按上游锁定的 SHA-256 取产品数据；只解压允许的相对路径。
#requires -Version 7.0
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$repository = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$lockLines = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'data.lock')
$tag = ($lockLines | Where-Object { $_ -match '^tag\s*=' }) -replace '^tag\s*=\s*', ''
$expected = ($lockLines | Where-Object { $_ -match '^qingjian-data.tar.gz\s*=' }) -replace '^qingjian-data.tar.gz\s*=\s*', ''
if ($tag -notmatch '^data-v\d+$' -or $expected -notmatch '^[a-f0-9]{64}$') { throw '无效的数据锁文件' }
$output = Join-Path $repository 'target/release-data'
New-Item -ItemType Directory -Path $output -Force | Out-Null
$archive = Join-Path $output 'qingjian-data.tar.gz'
$url = "https://github.com/qingjian-team/qingjian/releases/download/$tag/qingjian-data.tar.gz"
if (-not (Test-Path -LiteralPath $archive)) {
    & curl.exe --fail --location --connect-timeout 15 --max-time 600 --output $archive $url
    if ($LASTEXITCODE -ne 0) {
        & curl.exe --fail --location --proxy http://127.0.0.1:7897 --connect-timeout 15 --max-time 600 --output $archive $url
        if ($LASTEXITCODE -ne 0) { throw '下载产品数据失败' }
    }
}
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) { throw '产品数据 SHA-256 不匹配，请移走损坏缓存再重试' }
$entries = & tar.exe -tzf $archive
if ($LASTEXITCODE -ne 0) { throw '读取归档目录失败' }
foreach ($entry in $entries) {
    $relative = $entry -replace '^\./', ''
    # 已锁定哈希的归档含顶层目录条目；只允许这个精确目录名。
    if ($relative -eq 'data/') { continue }
    if ($relative -notmatch '^data/(generated|models)/' -or $relative -match '(^|/)\.\.(/|$)|\\|:' ) { throw "拒绝异常归档路径：$entry" }
}
& tar.exe -xzf $archive -C $repository
if ($LASTEXITCODE -ne 0) { throw '解压产品数据失败' }
Write-Host "已验证上游数据 $tag / $expected"
