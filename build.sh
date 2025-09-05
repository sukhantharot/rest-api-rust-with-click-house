#!/bin/bash

# Build script for REST API Rust ClickHouse

set -e  # Exit on any error

echo "🏗️  Building REST API Docker Image..."

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    exit 1
fi

# Build the Docker image
echo "📦 Building Docker image..."
docker build -t rarch:latest .

echo "✅ Docker image built successfully!"

# Optional: Run the container
read -p "🚀 Do you want to run the container now? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "🌟 Starting the container..."
    docker-compose up -d
    echo "✅ Container started! API is available at http://localhost:3700"
    echo "📊 Check logs with: docker-compose logs -f rest-api"
    echo "🛑 Stop with: docker-compose down"
else
    echo "🐳 To run later, use: docker-compose up -d"
fi

echo "🎉 Build completed!"
