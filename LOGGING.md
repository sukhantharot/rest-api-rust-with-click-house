# 📊 Request Logging & Performance Monitoring Guide

## Overview

The REST API includes comprehensive request logging and performance monitoring capabilities that track every API call with detailed timing information, request/response data, and performance metrics.

## 🚀 Quick Start

### Enable Logging
Set environment variables in your `.env` file:

```env
# Basic Logging
LOG_LEVEL=info
LOG_REQUEST_LOGGING=true
LOG_SLOW_REQUEST_THRESHOLD_MS=1000

# Advanced Logging
LOG_JSON_FORMAT=false
LOG_PERFORMANCE_METRICS=true
LOG_FILE_LOGGING=false
LOG_FILE_PATH=logs/app.log
```

### Run with Logging
```bash
# Start server with logging
cargo run

# Or with Docker
docker-compose up -d

# View logs in real-time
docker-compose logs -f rest-api
```

## 📋 Configuration Options

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `LOG_LEVEL` | `info` | Log level: trace, debug, info, warn, error |
| `LOG_REQUEST_LOGGING` | `true` | Enable/disable request logging |
| `LOG_JSON_FORMAT` | `false` | Use JSON format for structured logging |
| `LOG_SLOW_REQUEST_THRESHOLD_MS` | `1000` | Threshold for slow request warnings (ms) |
| `LOG_PERFORMANCE_METRICS` | `true` | Include performance metrics in logs |
| `LOG_FILE_LOGGING` | `false` | Enable file logging |
| `LOG_FILE_PATH` | `logs/app.log` | Path for log files |

### Advanced Configuration

```env
# Detailed logging targets
RUST_LOG="rest_api_rust_clickhouse=debug,tower_http=info,axum=debug,clickhouse=warn"

# Production settings
LOG_JSON_FORMAT=true
LOG_LEVEL=warn
LOG_SLOW_REQUEST_THRESHOLD_MS=500
```

## 📊 Log Output Examples

### Standard Request Log
```
2024-01-15T10:30:45.123Z INFO [rest-api] 📥 Incoming request
  request_id: "f47ac10b-58cc-4372-a567-0e02b2c3d479"
  method: "GET"
  uri: "/users"
  path: "/users"
  client_ip: "192.168.1.100"
  user_agent: "Mozilla/5.0 ..."
  content_length: "0"

2024-01-15T10:30:45.156Z INFO [rest-api] ✅ Request completed
  request_id: "f47ac10b-58cc-4372-a567-0e02b2c3d479"
  method: "GET"
  path: "/users"
  status: "200"
  duration_ms: "33"
  client_ip: "192.168.1.100"
  response_size: "1024"
```

### Slow Request Warning
```
2024-01-15T10:31:20.567Z WARN [rest-api] 🐌 Slow request completed
  request_id: "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
  method: "POST"
  path: "/blogs"
  status: "201"
  duration_ms: "1250"
  client_ip: "192.168.1.100"
```

### Error Request Log
```
2024-01-15T10:32:10.890Z WARN [rest-api] ❌ Request completed with error
  request_id: "error-12345-67890"
  method: "GET"
  path: "/users/invalid"
  status: "404"
  duration_ms: "15"
  client_ip: "192.168.1.100"
```

### JSON Format Log
```json
{
  "timestamp": "2024-01-15T10:30:45.123Z",
  "level": "INFO",
  "fields": {
    "message": "✅ Request completed",
    "request_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
    "method": "GET",
    "path": "/users",
    "status": "200",
    "duration_ms": "33",
    "client_ip": "192.168.1.100",
    "response_size": "1024"
  },
  "target": "rest_api_rust_clickhouse"
}
```

## 🎯 Features

### 1. Request Timing
- **Precise timing**: Microsecond precision for request duration
- **Slow request detection**: Configurable threshold warnings
- **Response time headers**: `x-response-time` header in responses

### 2. Request Tracking
- **Unique request IDs**: UUID for each request
- **Request correlation**: `x-request-id` header passed through
- **Distributed tracing**: Compatible with APM systems

### 3. Comprehensive Metrics
- Request method and path
- Response status codes
- Client IP addresses
- User agent strings
- Request/response sizes
- Performance metrics

### 4. Structured Logging
- **JSON format support**: For log aggregation systems
- **Structured fields**: Easy parsing and filtering
- **Multiple log levels**: Fine-grained control

## 📈 Performance Metrics

### Automatic Metrics Collection
```
2024-01-15T10:30:45.156Z DEBUG [rest-api] 📊 Performance metrics
  request_id: "f47ac10b-58cc-4372-a567-0e02b2c3d479"
  performance.duration_ms: "33"
  performance.request_size_bytes: "256"
  performance.response_size_bytes: "1024"
  performance.requests_per_second: "30.30"
```

### Response Headers
Every response includes performance headers:
```http
x-request-id: f47ac10b-58cc-4372-a567-0e02b2c3d479
x-response-time: 33ms
```

## 🔧 Integration

### With Log Aggregation Systems
**ELK Stack (Elasticsearch, Logstash, Kibana):**
```env
LOG_JSON_FORMAT=true
LOG_LEVEL=info
```

**Fluentd:**
```env
LOG_JSON_FORMAT=true
LOG_FILE_LOGGING=true
LOG_FILE_PATH=/var/log/app/rest-api.log
```

**DataDog/New Relic:**
```env
LOG_JSON_FORMAT=true
LOG_PERFORMANCE_METRICS=true
```

### With APM Systems
The request IDs and structured logging work with:
- Jaeger
- Zipkin  
- OpenTelemetry
- DataDog APM
- New Relic

## 🐳 Docker Configuration

### Basic Setup
```yaml
# docker-compose.yml
services:
  rest-api:
    build: .
    environment:
      - LOG_LEVEL=info
      - LOG_REQUEST_LOGGING=true
      - LOG_JSON_FORMAT=false
    volumes:
      - ./logs:/app/logs
```

### Production Setup
```yaml
services:
  rest-api:
    build: .
    environment:
      - LOG_LEVEL=warn
      - LOG_REQUEST_LOGGING=true
      - LOG_JSON_FORMAT=true
      - LOG_SLOW_REQUEST_THRESHOLD_MS=500
      - LOG_FILE_LOGGING=true
    volumes:
      - ./logs:/app/logs
    logging:
      driver: "json-file"
      options:
        max-size: "100m"
        max-file: "3"
```

## 📊 Monitoring Dashboards

### Key Metrics to Track
1. **Request Rate**: Requests per second
2. **Response Time**: P50, P95, P99 percentiles  
3. **Error Rate**: 4xx and 5xx responses
4. **Slow Requests**: Above threshold warnings
5. **Top Endpoints**: Most frequently accessed

### Sample Queries

**Grafana/Prometheus:**
```promql
# Request rate
sum(rate(http_requests_total[5m]))

# Average response time
avg(http_request_duration_seconds)

# Error rate
sum(rate(http_requests_total{status=~"4..|5.."}[5m]))
```

**Elasticsearch:**
```json
{
  "query": {
    "range": {
      "fields.duration_ms": {"gte": 1000}
    }
  }
}
```

## 🔍 Debugging & Troubleshooting

### Enable Debug Logging
```env
LOG_LEVEL=debug
RUST_LOG="rest_api_rust_clickhouse=debug,tower_http=debug"
```

### Common Issues

**High Response Times:**
1. Check slow request logs
2. Monitor database connection times
3. Review ClickHouse query performance

**Missing Request IDs:**
1. Verify `LOG_REQUEST_LOGGING=true`
2. Check middleware configuration
3. Ensure headers are preserved

**Log Volume Too High:**
1. Increase `LOG_SLOW_REQUEST_THRESHOLD_MS`
2. Set `LOG_LEVEL=warn` or `error`
3. Disable `LOG_PERFORMANCE_METRICS`

## 🎛️ Log Rotation

### File-based Logging
```env
LOG_FILE_LOGGING=true
LOG_FILE_PATH=logs/app.log

# Rotation settings (future enhancement)
LOG_ROTATION_ENABLED=true
LOG_MAX_FILE_SIZE_MB=100
LOG_MAX_FILES=10
```

### Docker Logging
```yaml
logging:
  driver: "json-file"
  options:
    max-size: "100m"
    max-file: "3"
```

## 🚀 Best Practices

### Development
- Use pretty format: `LOG_JSON_FORMAT=false`
- Enable debug logging: `LOG_LEVEL=debug`
- Lower slow threshold: `LOG_SLOW_REQUEST_THRESHOLD_MS=100`

### Production
- Use JSON format: `LOG_JSON_FORMAT=true`
- Set appropriate level: `LOG_LEVEL=info` or `warn`
- Higher slow threshold: `LOG_SLOW_REQUEST_THRESHOLD_MS=1000`
- Enable file logging for persistence

### Performance
- Disable detailed metrics if not needed
- Use log aggregation systems for analysis
- Monitor log volume and storage

## 🔒 Security & Privacy

### PII Protection
- User agent strings are logged (consider truncating)
- IP addresses are logged (consider anonymizing)
- Request bodies are NOT logged
- Response bodies are NOT logged

### Log Sanitization
```rust
// Custom middleware can be added to sanitize sensitive data
// Headers like Authorization are automatically excluded
```

## 📞 Support

For issues with logging:
1. Check environment variables
2. Verify middleware configuration
3. Review log output format
4. Test with minimal configuration
5. Check file permissions (if using file logging)

## 🔄 Future Enhancements

Planned features:
- [ ] Automatic log rotation
- [ ] Metrics export (Prometheus)
- [ ] Custom log formatters
- [ ] Log sampling for high-traffic
- [ ] Integration with OpenTelemetry
- [ ] Request body logging (optional)
- [ ] Custom log filters
