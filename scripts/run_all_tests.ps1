# Agentic-Warden Windows 测试运行器
# PowerShell版本的测试运行脚本

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("unit", "integration", "cli", "tui", "performance", "coverage", "all", "quick", "smoke")]
    [string]$TestType = "all",

    [Parameter(Mandatory=$false)]
    [int]$Timeout = 300,

    [Parameter(Mandatory=$false)]
    [string]$OutputDir = "test-results",

    [Parameter(Mandatory=$false)]
    [switch]$Verbose,

    [Parameter(Mandatory=$false)]
    [switch]$Quiet,

    [Parameter(Mandatory=$false)]
    [switch]$NoFailFast,

    [Parameter(Mandatory=$false)]
    [switch]$Help
)

# 颜色输出函数
function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$Color = "White"
    )

    $colors = @{
        "Red" = "Red"
        "Green" = "Green"
        "Yellow" = "Yellow"
        "Blue" = "Blue"
        "White" = "White"
    }

    Write-Host $Message -ForegroundColor $colors[$Color]
}

function Write-Info {
    param([string]$Message)
    if (-not $Quiet) {
        Write-ColorOutput "[INFO] $Message" "Blue"
    }
}

function Write-Success {
    param([string]$Message)
    if (-not $Quiet) {
        Write-ColorOutput "[SUCCESS] $Message" "Green"
    }
}

function Write-Warning {
    param([string]$Message)
    Write-ColorOutput "[WARNING] $Message" "Yellow"
}

function Write-Error {
    param([string]$Message)
    Write-ColorOutput "[ERROR] $Message" "Red"
}

function Show-Help {
    @"
Agentic-Warden Windows 测试运行器

用法: .\run_all_tests.ps1 [选项]

选项:
  -TestType <类型>      测试类型 (unit, integration, cli, tui, performance, coverage, all, quick, smoke)
  -Timeout <秒>        测试超时时间 (默认: 300)
  -OutputDir <路径>    输出目录 (默认: test-results)
  -Verbose             详细输出
  -Quiet               静默模式
  -NoFailFast          遇到失败时继续运行
  -Help                显示此帮助信息

示例:
  .\run_all_tests.ps1                    # 运行所有测试
  .\run_all_tests.ps1 -TestType unit      # 只运行单元测试
  .\run_all_tests.ps1 -Verbose            # 详细输出
  .\run_all_tests.ps1 -Timeout 600 all    # 设置超时时间
"@
}

function Set-TestEnvironment {
    Write-Info "设置测试环境..."

    # 创建输出目录
    if (-not (Test-Path $OutputDir)) {
        New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
    }

    # 设置环境变量
    $env:AGENTIC_WARDEN_TEST_MODE = "1"
    $env:SKIP_NETWORK_CALLS = "1"
    $env:RUST_LOG = "debug"
    $env:RUST_BACKTRACE = "1"

    Write-Success "测试环境设置完成"
}

function Invoke-UnitTest {
    Write-Info "运行单元测试..."

    $testArgs = @("test", "--lib")
    if ($Verbose) {
        $testArgs += "--", "--nocapture"
    }
    if (-not $NoFailFast) {
        $testArgs += "--", "--no-fail-fast"
    }

    $process = Start-Process -FilePath "cargo" -ArgumentList $testArgs -Wait -PassThru -RedirectStandardOutput "$OutputDir\unit_tests.log" -RedirectStandardError "$OutputDir\unit_tests_error.log"

    if ($process.ExitCode -eq 0) {
        Write-Success "单元测试通过"
        return $true
    } else {
        Write-Error "单元测试失败"
        return $false
    }
}

function Invoke-IntegrationTest {
    Write-Info "运行集成测试..."

    $testArgs = @("test", "--test", "integration")
    if ($Verbose) {
        $testArgs += "--", "--nocapture"
    }
    if (-not $NoFailFast) {
        $testArgs += "--", "--no-fail-fast"
    }

    $process = Start-Process -FilePath "cargo" -ArgumentList $testArgs -Wait -PassThru -RedirectStandardOutput "$OutputDir\integration_tests.log" -RedirectStandardError "$OutputDir\integration_tests_error.log"

    if ($process.ExitCode -eq 0) {
        Write-Success "集成测试通过"
        return $true
    } else {
        Write-Error "集成测试失败"
        return $false
    }
}

function Invoke-CliTest {
    Write-Info "运行CLI测试..."

    $testArgs = @("test", "--test", "cli_integration")
    if ($Verbose) {
        $testArgs += "--", "--nocapture"
    }

    $process = Start-Process -FilePath "cargo" -ArgumentList $testArgs -Wait -PassThru -RedirectStandardOutput "$OutputDir\cli_tests.log" -RedirectStandardError "$OutputDir\cli_tests_error.log"

    if ($process.ExitCode -eq 0) {
        Write-Success "CLI测试通过"
        return $true
    } else {
        Write-Error "CLI测试失败"
        return $false
    }
}

function Invoke-TuiTest {
    Write-Info "运行TUI测试..."

    $testArgs = @("test", "--test", "tui_integration")
    if ($Verbose) {
        $testArgs += "--", "--nocapture"
    }

    # 设置TUI测试环境
    $env:TERM = "xterm-256color"

    $process = Start-Process -FilePath "cargo" -ArgumentList $testArgs -Wait -PassThru -RedirectStandardOutput "$OutputDir\tui_tests.log" -RedirectStandardError "$OutputDir\tui_tests_error.log"

    if ($process.ExitCode -eq 0) {
        Write-Success "TUI测试通过"
        return $true
    } else {
        Write-Error "TUI测试失败"
        return $false
    }
}

function Invoke-PerformanceTest {
    Write-Info "运行性能测试..."

    # 检查是否安装了cargo-criterion
    try {
        $null = Get-Command "cargo-criterion" -ErrorAction Stop
    }
    catch {
        Write-Warning "cargo-criterion未安装，跳过性能测试"
        return $true
    }

    $process = Start-Process -FilePath "cargo" -ArgumentList @("criterion") -Wait -PassThru -RedirectStandardOutput "$OutputDir\performance_tests.log" -RedirectStandardError "$OutputDir\performance_tests_error.log"

    # 性能测试失败不应该阻止CI
    Write-Success "性能测试完成"
    return $true
}

function New-CoverageReport {
    Write-Info "生成代码覆盖率报告..."

    # 检查是否安装了cargo-llvm-cov
    try {
        $null = Get-Command "cargo-llvm-cov" -ErrorAction Stop
    }
    catch {
        Write-Warning "cargo-llvm-cov未安装，跳过覆盖率报告"
        return $true
    }

    $process = Start-Process -FilePath "cargo" -ArgumentList @("llvm-cov", "--workspace", "--lcov", "--output-path", "$OutputDir\lcov.info", "--html", "--output-dir", "$OutputDir\coverage") -Wait -PassThru -RedirectStandardOutput "$OutputDir\coverage.log" -RedirectStandardError "$OutputDir\coverage_error.log"

    if ($process.ExitCode -eq 0) {
        Write-Success "覆盖率报告生成完成: $OutputDir\coverage"
        return $true
    } else {
        Write-Error "覆盖率报告生成失败"
        return $false
    }
}

function Invoke-QuickTest {
    Write-Info "运行快速测试套件..."

    $testArgs = @("test", "--lib", "--test", "cli_integration", "--", "--skip", "slow")
    if ($Verbose) {
        $testArgs += "--", "--nocapture"
    }

    $process = Start-Process -FilePath "cargo" -ArgumentList $testArgs -Wait -PassThru -RedirectStandardOutput "$OutputDir\quick_tests.log" -RedirectStandardError "$OutputDir\quick_tests_error.log"

    if ($process.ExitCode -eq 0) {
        Write-Success "快速测试通过"
        return $true
    } else {
        Write-Error "快速测试失败"
        return $false
    }
}

function Invoke-SmokeTest {
    Write-Info "运行冒烟测试..."

    # 构建项目
    $buildProcess = Start-Process -FilePath "cargo" -ArgumentList @("build", "--release") -Wait -PassThru -RedirectStandardOutput "$OutputDir\build.log" -RedirectStandardError "$OutputDir\build_error.log"

    if ($buildProcess.ExitCode -ne 0) {
        Write-Error "项目构建失败"
        return $false
    }

    # 测试基本CLI功能
    $binaryPath = ".\target\release\agentic-warden.exe"
    if (-not (Test-Path $binaryPath)) {
        Write-Error "可执行文件不存在: $binaryPath"
        return $false
    }

    $versionTest = & $binaryPath "--version"
    if ($LASTEXITCODE -ne 0) {
        Write-Error "版本命令失败"
        return $false
    }

    $helpTest = & $binaryPath "--help"
    if ($LASTEXITCODE -ne 0) {
        Write-Error "帮助命令失败"
        return $false
    }

    Write-Success "冒烟测试通过"
    return $true
}

function New-TestReport {
    Write-Info "生成测试报告..."

    $reportFile = Join-Path $OutputDir "test_report.md"

    $reportContent = @"
# Agentic-Warden 测试报告

## 测试配置
- 测试类型: $TestType
- 超时时间: ${Timeout}s
- 运行时间: $(Get-Date)
- Git提交: $((git rev-parse --short HEAD 2>$null) -replace '`', '')

## 测试结果
"@

    # 添加各测试结果
    $testLogs = @("unit_tests.log", "integration_tests.log", "cli_tests.log", "tui_tests.log", "quick_tests.log")
    foreach ($testLog in $testLogs) {
        $logPath = Join-Path $OutputDir $testLog
        if (Test-Path $logPath) {
            $testName = $testLog -replace "\.log$", ""
            $reportContent += @"

### $($testName -replace "_", " ").ToUpper()
```
$(Get-Content $logPath | Select-Object -Last 20)
```

"@
        }
    }

    # 添加覆盖率信息
    $lcovPath = Join-Path $OutputDir "lcov.info"
    if (Test-Path $lcovPath) {
        $reportContent += @"

## 代码覆盖率
- HTML报告: [查看详情](coverage/index.html)
- LCOV文件: lcov.info

"@
    }

    $reportContent | Out-File -FilePath $reportFile -Encoding UTF8

    Write-Success "测试报告生成完成: $reportFile"
}

# 主函数
function Main {
    if ($Help) {
        Show-Help
        return 0
    }

    Set-TestEnvironment

    $exitCode = 0

    # 根据测试类型运行相应的测试
    switch ($TestType) {
        "unit" {
            if (-not (Invoke-UnitTest)) { $exitCode = 1 }
        }
        "integration" {
            if (-not (Invoke-IntegrationTest)) { $exitCode = 1 }
        }
        "cli" {
            if (-not (Invoke-CliTest)) { $exitCode = 1 }
        }
        "tui" {
            if (-not (Invoke-TuiTest)) { $exitCode = 1 }
        }
        "performance" {
            if (-not (Invoke-PerformanceTest)) { $exitCode = 1 }
        }
        "coverage" {
            if (-not (New-CoverageReport)) { $exitCode = 1 }
        }
        "quick" {
            if (-not (Invoke-QuickTest)) { $exitCode = 1 }
        }
        "smoke" {
            if (-not (Invoke-SmokeTest)) { $exitCode = 1 }
        }
        "all" {
            Write-Info "运行所有测试..."

            if (-not (Invoke-UnitTest)) { $exitCode = 1 }
            if (-not (Invoke-IntegrationTest)) { $exitCode = 1 }
            if (-not (Invoke-CliTest)) { $exitCode = 1 }
            if (-not (Invoke-TuiTest)) { $exitCode = 1 }

            # 性能测试失败不应该阻止CI
            Invoke-PerformanceTest | Out-Null

            # 覆盖率报告失败不应该阻止CI
            New-CoverageReport | Out-Null
        }
        default {
            Write-Error "未知测试类型: $TestType"
            Show-Help
            return 1
        }
    }

    # 生成报告
    New-TestReport

    # 输出总结
    Write-Host "="*50
    Write-Host "📊 测试运行总结"
    Write-Host "="*50

    if ($exitCode -eq 0) {
        Write-Success "所有测试通过! 🎉"
    } else {
        Write-Error "测试失败! 💥"
    }

    return $exitCode
}

# 脚本入口
try {
    $result = Main
    exit $result
}
catch {
    Write-Error "脚本执行失败: $($_.Exception.Message)"
    exit 1
}