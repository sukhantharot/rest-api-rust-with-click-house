# Build script for REST API Rust ClickHouse (PowerShell)

# Set error handling
$ErrorActionPreference = "Stop"

Write-Host "🏗️  Building REST API Docker Image..." -ForegroundColor Cyan

# Check if Docker is installed
try {
    docker --version | Out-Null
}
catch {
    Write-Host "❌ Docker is not installed. Please install Docker Desktop first." -ForegroundColor Red
    exit 1
}

try {
    # Build the Docker image
    Write-Host "📦 Building Docker image..." -ForegroundColor Yellow
    docker build -t rarch:latest .

    Write-Host "✅ Docker image built successfully!" -ForegroundColor Green

    # Optional: Run the container
    $response = Read-Host "🚀 Do you want to run the container now? (y/N)"
    if ($response -eq "y" -or $response -eq "Y") {
        Write-Host "🌟 Starting the container..." -ForegroundColor Yellow
        docker-compose up -d
        Write-Host "✅ Container started! API is available at http://localhost:3700" -ForegroundColor Green
        Write-Host "📊 Check logs with: docker-compose logs -f rest-api" -ForegroundColor Cyan
        Write-Host "🛑 Stop with: docker-compose down" -ForegroundColor Cyan
    }
    else {
        Write-Host "🐳 To run later, use: docker-compose up -d" -ForegroundColor Cyan
    }

    Write-Host "🎉 Build completed!" -ForegroundColor Green
}
catch {
    Write-Host "❌ Build failed: $_" -ForegroundColor Red
    exit 1
}
