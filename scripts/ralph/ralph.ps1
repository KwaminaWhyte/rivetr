# Ralph Wiggum - Long-running AI agent loop for Claude Code (Windows)
# Usage: .\ralph.ps1 [max_iterations]
#
# This is an adaptation of https://github.com/snarktank/ralph for Claude Code
# Based on Geoffrey Huntley's Ralph pattern: https://ghuntley.com/ralph/

param(
    [int]$MaxIterations = 10
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$PrdFile = Join-Path $ScriptDir "prd.json"
$ProgressFile = Join-Path $ScriptDir "progress.txt"
$ArchiveDir = Join-Path $ScriptDir "archive"
$LastBranchFile = Join-Path $ScriptDir ".last-branch"

# Archive previous run if branch changed
if ((Test-Path $PrdFile) -and (Test-Path $LastBranchFile)) {
    try {
        $prdContent = Get-Content $PrdFile -Raw | ConvertFrom-Json
        $currentBranch = $prdContent.branchName
        $lastBranch = Get-Content $LastBranchFile -Raw

        if ($currentBranch -and $lastBranch -and ($currentBranch -ne $lastBranch.Trim())) {
            # Archive the previous run
            $date = Get-Date -Format "yyyy-MM-dd"
            $folderName = $lastBranch.Trim() -replace "^ralph/", ""
            $archiveFolder = Join-Path $ArchiveDir "$date-$folderName"

            Write-Host "Archiving previous run: $lastBranch"
            New-Item -ItemType Directory -Path $archiveFolder -Force | Out-Null
            if (Test-Path $PrdFile) { Copy-Item $PrdFile $archiveFolder }
            if (Test-Path $ProgressFile) { Copy-Item $ProgressFile $archiveFolder }
            Write-Host "   Archived to: $archiveFolder"

            # Reset progress file for new run
            @"
# Ralph Progress Log
Started: $(Get-Date)
---
"@ | Set-Content $ProgressFile
        }
    }
    catch {
        Write-Warning "Could not check branch: $_"
    }
}

# Track current branch
if (Test-Path $PrdFile) {
    try {
        $prdContent = Get-Content $PrdFile -Raw | ConvertFrom-Json
        if ($prdContent.branchName) {
            $prdContent.branchName | Set-Content $LastBranchFile
        }
    }
    catch {
        Write-Warning "Could not read PRD file: $_"
    }
}

# Initialize progress file if it doesn't exist
if (-not (Test-Path $ProgressFile)) {
    @"
# Ralph Progress Log
Started: $(Get-Date)
---
"@ | Set-Content $ProgressFile
}

Write-Host "Starting Ralph - Max iterations: $MaxIterations"
Write-Host "Using Claude Code as the AI backend"

$promptFile = Join-Path $ScriptDir "prompt.md"

for ($i = 1; $i -le $MaxIterations; $i++) {
    Write-Host ""
    Write-Host "==============================================================="
    Write-Host "  Ralph Iteration $i of $MaxIterations"
    Write-Host "==============================================================="

    # Run Claude Code with the ralph prompt
    try {
        $promptContent = Get-Content $promptFile -Raw
        $output = $promptContent | claude --dangerously-skip-permissions 2>&1 | Tee-Object -Variable capturedOutput
        Write-Host $output
    }
    catch {
        Write-Warning "Iteration error: $_"
        $output = ""
    }

    # Check for completion signal
    if ($output -match "<promise>COMPLETE</promise>") {
        Write-Host ""
        Write-Host "Ralph completed all tasks!"
        Write-Host "Completed at iteration $i of $MaxIterations"
        exit 0
    }

    Write-Host "Iteration $i complete. Continuing..."
    Start-Sleep -Seconds 2
}

Write-Host ""
Write-Host "Ralph reached max iterations ($MaxIterations) without completing all tasks."
Write-Host "Check $ProgressFile for status."
exit 1
