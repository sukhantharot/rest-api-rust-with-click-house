# 🗃️ Database Migrations Guide

## Overview

The REST API supports two types of migrations:
- **Base Database Migrations**: Creates system tables (`client_connect`, `users`) in the main database
- **Client Database Migrations**: Creates tenant-specific tables for each client domain

## 🚀 Quick Start

### 1. Environment Setup
Create a `.env` file with your database configuration:
```env
CLICKHOUSE_URL=https://user:password@host:port/database
DATABASE_NAME=your_database_name
JWT_SECRET=your-jwt-secret
```

### 2. Run All Migrations (Recommended)
```bash
# Linux/Mac
./scripts/migrate.sh all

# Windows PowerShell  
.\scripts\migrate.ps1 all
```

## 📋 Migration Commands

### Base Database Migration
Creates core system tables in the main database:

**Local Development:**
```bash
# Build and run directly
cargo build --bin migrate --release
./target/release/migrate base

# Using scripts
./scripts/migrate.sh base              # Linux/Mac
.\scripts\migrate.ps1 base             # Windows
```

**Docker:**
```bash
# Run in existing container
docker-compose exec rest-api ./migrate base

# Run one-time migration container
docker build -f docker/Dockerfile.migrations -t migrations .
docker run --rm -e CLICKHOUSE_URL="your-url" migrations base
```

### Client Database Migration

**By Database URL:**
```bash
./target/release/migrate client "https://user:pass@host:port/client_db"

# Using scripts
./scripts/migrate.sh client "https://user:pass@host:port/client_db"
.\scripts\migrate.ps1 client "https://user:pass@host:port/client_db"
```

**By Domain Name:**
```bash
./target/release/migrate client --domain example.com

# Using scripts  
./scripts/migrate.sh client-domain example.com
.\scripts\migrate.ps1 client-domain example.com
```

**Docker:**
```bash
docker-compose exec rest-api ./migrate client --domain example.com
```

### All Migrations
Runs base migration first, then all client migrations:

```bash
./target/release/migrate all
./scripts/migrate.sh all
.\scripts\migrate.ps1 all
docker-compose exec rest-api ./migrate all
```

## 🌐 Using Admin API

You can also run migrations through the REST API endpoints:

### 1. Login as Admin
```bash
curl -X POST http://localhost:3000/admin/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username_or_email": "admin",
    "password": "admin123"
  }'
```

### 2. Run Migration for Client
```bash
curl -X POST http://localhost:3000/admin/clients/{client_id}/migrate \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json"
```

## 🐳 Docker Strategies

### Strategy 1: Init Container
Add an init container to run migrations before starting the main app:

```yaml
# docker-compose.yml
services:
  migrate:
    build: .
    command: ./migrate all
    environment:
      - CLICKHOUSE_URL=${CLICKHOUSE_URL}
    depends_on:
      - base-setup

  rest-api:
    build: .
    depends_on:
      - migrate
    # ... other config
```

### Strategy 2: Startup Script
Modify your main container to run migrations on startup:

```dockerfile
# Create startup script
COPY scripts/startup.sh /app/startup.sh
RUN chmod +x /app/startup.sh
CMD ["/app/startup.sh"]
```

```bash
# scripts/startup.sh
#!/bin/bash
./migrate base
./migrate all
exec ./rest-api-rust-clickhouse
```

### Strategy 3: Manual Migration
Run migrations manually before starting the application:

```bash
# Build and run migration container
docker build -t rest-api .
docker run --rm -e CLICKHOUSE_URL="$CLICKHOUSE_URL" rest-api ./migrate all

# Then start the main application
docker-compose up -d
```

## 📊 Database Schema

### Base Database Tables

**client_connect:**
- `id`: UInt64 - Unique client identifier
- `domain`: String - Client domain name
- `database_url`: String - Client's ClickHouse connection URL  
- `database_name`: String - Client's database name
- `is_active`: Bool - Whether client is active
- `created_at`: DateTime - Creation timestamp
- `updated_at`: DateTime - Last update timestamp

**users (Base Admin Users):**
- `id`: String - Unique user identifier
- `username`: String - Admin username
- `email`: String - Admin email address
- `password_hash`: String - Hashed password
- `first_name`: Nullable(String) - First name
- `last_name`: Nullable(String) - Last name
- `is_active`: Bool - Whether user is active
- `is_verified`: Bool - Whether email is verified
- `role`: Nullable(String) - User role
- `last_login_at`: Nullable(DateTime) - Last login time
- `created_at`: DateTime - Creation timestamp
- `updated_at`: DateTime - Last update timestamp

### Client Database Tables
Each client database contains:
- `users` - Client-specific users
- `roles` - User roles
- `permissions` - Role permissions  
- `blog` - Blog posts
- `tag` - Blog tags
- `blog_category` - Blog categories
- `auth_tracking` - Authentication logs
- `blog_tracking` - Blog interaction logs

## 🔧 Troubleshooting

### Common Issues

**Connection Error:**
```
Error: Failed to connect to ClickHouse
```
- Check your `CLICKHOUSE_URL` environment variable
- Verify network connectivity to ClickHouse server
- Confirm credentials are correct

**Permission Error:**
```
Error: Access denied for user
```
- Verify database user has CREATE TABLE permissions
- Check if user can access the specified database

**Migration Already Exists:**
```
Table already exists
```
- This is normal - migrations use `CREATE TABLE IF NOT EXISTS`
- Tables won't be recreated if they already exist

### Debug Mode
Enable debug logging:
```bash
RUST_LOG=debug ./target/release/migrate all
```

### Rollback Strategy
Since ClickHouse doesn't support traditional rollbacks:
1. **Backup your data** before running migrations
2. Use `DROP TABLE` commands if you need to remove tables
3. Re-run migrations to recreate tables

### Manual Verification
Check if tables were created:
```sql
-- Base database
SHOW TABLES;
DESCRIBE client_connect;
DESCRIBE users;

-- Client database  
SHOW TABLES;
DESCRIBE users;
DESCRIBE roles;
-- etc.
```

## 🔄 CI/CD Integration

### GitHub Actions Example
```yaml
name: Deploy with Migrations

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Run Migrations
        run: |
          docker build -t migrations -f docker/Dockerfile.migrations .
          docker run --rm \
            -e CLICKHOUSE_URL="${{ secrets.CLICKHOUSE_URL }}" \
            migrations all
            
      - name: Deploy Application
        run: |
          docker-compose up -d
```

### Manual Production Deployment
```bash
# 1. Build migration image
docker build -t migrations -f docker/Dockerfile.migrations .

# 2. Run migrations
docker run --rm \
  -e CLICKHOUSE_URL="$PRODUCTION_CLICKHOUSE_URL" \
  migrations all

# 3. Deploy application
docker-compose -f docker-compose.prod.yml up -d
```

## 🎯 Best Practices

1. **Always backup** before running migrations in production
2. **Test migrations** in a staging environment first
3. **Run base migrations** before client migrations
4. **Use CI/CD** to automate migration deployment
5. **Monitor logs** during migration execution
6. **Use init containers** in production Docker deployments
7. **Set proper timeouts** for long-running migrations

## 📞 Support

If you encounter issues:
1. Check the troubleshooting section above
2. Enable debug logging (`RUST_LOG=debug`)
3. Verify your environment variables
4. Test connectivity to ClickHouse manually
5. Review the migration source code in `src/migrations/`
