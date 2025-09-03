# 🐳 Docker Setup Guide

## Quick Start

### 🚀 Option 1: Auto Build & Run
**Windows (PowerShell):**
```powershell
.\build.ps1
```

**Linux/Mac (Bash):**
```bash
chmod +x build.sh
./build.sh
```

### 🏗️ Option 2: Manual Docker Commands

#### Build the image:
```bash
docker build -t rest-api-rust-clickhouse:latest .
```

#### Run with Docker Compose:
```bash
docker-compose up -d
```

#### Run with Docker directly:
```bash
docker run -d \
  --name rest-api \
  -p 3000:3000 \
  -e CLICKHOUSE_URL="https://clickhouse:vOn8UIeaAdx3Rgz7wRYuMRlUiaHWBWhg@clickhouse-production-71f9.up.railway.app:443/railway" \
  -e JWT_SECRET="your-secret-key" \
  rest-api-rust-clickhouse:latest
```

## 🌐 Environment Variables

### Required Variables:
- `CLICKHOUSE_URL`: ClickHouse database connection URL
- `JWT_SECRET`: Secret key for JWT tokens

### Optional Variables:
- `RUST_LOG`: Log level (default: `info`)
- `SERVER_HOST`: Server bind address (default: `0.0.0.0`)
- `SERVER_PORT`: Server port (default: `3000`)
- `JWT_EXPIRATION_HOURS`: JWT token expiration (default: `24`)

### Example .env file:
```env
CLICKHOUSE_URL=https://clickhouse:password@your-host:443/database
JWT_SECRET=your-super-secret-jwt-key
RUST_LOG=info
SERVER_PORT=3000
```

## 📊 Management Commands

### Check logs:
```bash
docker-compose logs -f rest-api
```

### Stop services:
```bash
docker-compose down
```

### Restart services:
```bash
docker-compose restart
```

### Remove everything:
```bash
docker-compose down -v
docker rmi rest-api-rust-clickhouse:latest
```

## 🔧 Troubleshooting

### Check if container is running:
```bash
docker ps
```

### Check container health:
```bash
docker-compose exec rest-api curl http://localhost:3000/health
```

### Access container shell:
```bash
docker-compose exec rest-api /bin/bash
```

### View container resource usage:
```bash
docker stats
```

## 🏭 Production Deployment

### 1. Use production environment:
```bash
docker-compose -f docker-compose.prod.yml up -d
```

### 2. Behind reverse proxy:
Uncomment the nginx service in `docker-compose.yml` and configure SSL certificates.

### 3. Resource limits:
Add resource constraints to your compose file:
```yaml
services:
  rest-api:
    # ... other config ...
    deploy:
      resources:
        limits:
          cpus: '0.5'
          memory: 512M
        reservations:
          cpus: '0.25'
          memory: 256M
```

## 🔒 Security Notes

1. **Never use default JWT secrets in production**
2. **Use proper SSL certificates**
3. **Limit container resources**
4. **Use non-root user (already configured)**
5. **Keep base images updated**

## 🏗️ Image Details

- **Base Image**: `debian:bookworm-slim`
- **Rust Version**: `1.75`
- **Final Image Size**: ~100MB (multi-stage build)
- **Security**: Runs as non-root user
- **Health Check**: Built-in health endpoint monitoring

## 📈 Monitoring

### Health Check Endpoint:
```bash
curl http://localhost:3000/health
```

### API Documentation:
```bash
curl http://localhost:3000/
```

### Admin APIs:
```bash
# Login to get admin token
curl -X POST http://localhost:3000/admin/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username_or_email":"admin","password":"admin123"}'

# List client connections
curl -H "Authorization: Bearer YOUR_TOKEN" \
  http://localhost:3000/admin/clients
```
