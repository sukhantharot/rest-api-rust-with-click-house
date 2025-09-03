# Migration script for REST API Rust ClickHouse (PowerShell)

param(
    [Parameter(Position = 0)]
    [string]$Command = "",
    [Parameter(Position = 1)]
    [string]$Parameter = ""
)

# Colors for output (PowerShell)
function Write-ColorOutput {
    param(
        [string]$Message,
        [ConsoleColor]$ForegroundColor = [ConsoleColor]::White
    )
    Write-Host $Message -ForegroundColor $ForegroundColor
}

Write-ColorOutput "🗃️  Database Migration Tool" -ForegroundColor Cyan
Write-ColorOutput "==========================" -ForegroundColor Cyan

# Load .env file if exists
if (Test-Path ".env") {
    Write-ColorOutput "✅ Loading .env file..." -ForegroundColor Green
    Get-Content .env | ForEach-Object {
        if ($_ -match "^([^#].*)=(.*)$") {
            $key = $matches[1].Trim()
            $value = $matches[2].Trim()
            [Environment]::SetEnvironmentVariable($key, $value, "Process")
        }
    }
}
else {
    Write-ColorOutput "⚠️  .env file not found. Using environment variables..." -ForegroundColor Yellow
}

# Check required environment variables
if (-not $env:CLICKHOUSE_URL) {
    Write-ColorOutput "❌ CLICKHOUSE_URL environment variable is required" -ForegroundColor Red
    exit 1
}

# Show usage if no command provided
if ([string]::IsNullOrEmpty($Command)) {
    Write-Host "Usage: .\migrate.ps1 <command> [options]"
    Write-Host ""
    Write-Host "Commands:"
    Write-Host "  base                     Run base database migrations"
    Write-Host "  client <database_url>    Run client migrations for specific database URL"
    Write-Host "  client-domain <domain>   Run client migrations for specific domain"
    Write-Host "  all                      Run all migrations (base + all clients)"
    Write-Host "  docker                   Run all migrations in Docker container"
    Write-Host "  help                     Show this help message"
    exit 0
}

try {
    # Build the migration binary
    Write-ColorOutput "🏗️  Building migration tool..." -ForegroundColor Blue
    cargo build --bin migrate --release

    $MigrateBin = ".\target\release\migrate.exe"

    # Check if binary exists
    if (-not (Test-Path $MigrateBin)) {
        Write-ColorOutput "❌ Migration binary not found at $MigrateBin" -ForegroundColor Red
        exit 1
    }

    switch ($Command) {
        "base" {
            Write-ColorOutput "🚀 Running Base Database Migrations..." -ForegroundColor Blue
            & $MigrateBin base
        }
        "client" {
            if ([string]::IsNullOrEmpty($Parameter)) {
                Write-ColorOutput "❌ Database URL required for client migration" -ForegroundColor Red
                Write-Host "Usage: .\migrate.ps1 client <database_url>"
                exit 1
            }
            Write-ColorOutput "🚀 Running Client Database Migrations..." -ForegroundColor Blue
            & $MigrateBin client $Parameter
        }
        "client-domain" {
            if ([string]::IsNullOrEmpty($Parameter)) {
                Write-ColorOutput "❌ Domain required for client migration" -ForegroundColor Red
                Write-Host "Usage: .\migrate.ps1 client-domain <domain>"
                exit 1
            }
            Write-ColorOutput "🚀 Running Client Database Migrations for domain: $Parameter" -ForegroundColor Blue
            & $MigrateBin client --domain $Parameter
        }
        "all" {
            Write-ColorOutput "🚀 Running All Migrations..." -ForegroundColor Blue
            & $MigrateBin all
        }
        "docker" {
            Write-ColorOutput "🐳 Running migrations in Docker container..." -ForegroundColor Blue
            docker-compose exec rest-api ./migrate all
        }
        "help" {
            & $MigrateBin help
        }
        default {
            Write-ColorOutput "❌ Unknown command: $Command" -ForegroundColor Red
            Write-Host "Use '.\migrate.ps1 help' for usage information"
            exit 1
        }
    }

    Write-ColorOutput "🎉 Migration completed successfully!" -ForegroundColor Green
}
catch {
    Write-ColorOutput "❌ Migration failed: $_" -ForegroundColor Red
    exit 1
}
