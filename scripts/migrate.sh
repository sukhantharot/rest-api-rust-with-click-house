#!/bin/bash

# Migration script for REST API Rust ClickHouse

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}🗃️  Database Migration Tool${NC}"
echo "=========================="

# Check if .env file exists
if [ ! -f .env ]; then
    echo -e "${YELLOW}⚠️  .env file not found. Using environment variables...${NC}"
else
    echo -e "${GREEN}✅ Loading .env file...${NC}"
    export $(grep -v '^#' .env | xargs)
fi

# Check required environment variables
if [ -z "$CLICKHOUSE_URL" ]; then
    echo -e "${RED}❌ CLICKHOUSE_URL environment variable is required${NC}"
    exit 1
fi

# Build the migration binary
echo -e "${BLUE}🏗️  Building migration tool...${NC}"
cargo build --bin migrate --release

MIGRATE_BIN="./target/release/migrate"

# Check if binary exists
if [ ! -f "$MIGRATE_BIN" ]; then
    echo -e "${RED}❌ Migration binary not found at $MIGRATE_BIN${NC}"
    exit 1
fi

# Parse command line arguments
if [ $# -eq 0 ]; then
    echo "Usage: $0 <command> [options]"
    echo ""
    echo "Commands:"
    echo "  base                     Run base database migrations"
    echo "  client <database_url>    Run client migrations for specific database URL"
    echo "  client-domain <domain>   Run client migrations for specific domain"
    echo "  all                      Run all migrations (base + all clients)"
    echo "  help                     Show this help message"
    exit 0
fi

COMMAND=$1

case $COMMAND in
    "base")
        echo -e "${BLUE}🚀 Running Base Database Migrations...${NC}"
        $MIGRATE_BIN base
        ;;
    "client")
        if [ -z "$2" ]; then
            echo -e "${RED}❌ Database URL required for client migration${NC}"
            echo "Usage: $0 client <database_url>"
            exit 1
        fi
        echo -e "${BLUE}🚀 Running Client Database Migrations...${NC}"
        $MIGRATE_BIN client "$2"
        ;;
    "client-domain")
        if [ -z "$2" ]; then
            echo -e "${RED}❌ Domain required for client migration${NC}"
            echo "Usage: $0 client-domain <domain>"
            exit 1
        fi
        echo -e "${BLUE}🚀 Running Client Database Migrations for domain: $2${NC}"
        $MIGRATE_BIN client --domain "$2"
        ;;
    "all")
        echo -e "${BLUE}🚀 Running All Migrations...${NC}"
        $MIGRATE_BIN all
        ;;
    "docker")
        echo -e "${BLUE}🐳 Running migrations in Docker container...${NC}"
        docker-compose exec rest-api /app/rest-api-rust-clickhouse migrate all
        ;;
    "help"|"--help"|"-h")
        $MIGRATE_BIN help
        ;;
    *)
        echo -e "${RED}❌ Unknown command: $COMMAND${NC}"
        echo "Use '$0 help' for usage information"
        exit 1
        ;;
esac

echo -e "${GREEN}🎉 Migration completed successfully!${NC}"
