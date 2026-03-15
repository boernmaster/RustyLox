# Phase 7 Plan: Production Hardening, Security & Scalability

<div align="center">

![Status](https://img.shields.io/badge/Status-Future-lightgrey)
![Phase](https://img.shields.io/badge/Phase-7-blue)
![Priority](https://img.shields.io/badge/Priority-Enterprise%20Ready-purple)

</div>

## Overview

Phase 7 focuses on enterprise-grade features:
- Production hardening and security
- High availability and scalability
- Performance optimization
- Security auditing and compliance
- Cloud deployment support
- Multi-instance and clustering
- Advanced authentication

## 1. Security Hardening (Priority: CRITICAL)

### 1.1 Authentication & Authorization

**Current State**: Basic or no auth
**Target**: Role-based access control (RBAC)

#### User Management System

```rust
pub struct User {
    id: Uuid,
    username: String,
    password_hash: String,  // Argon2 hashed
    email: String,
    roles: Vec<Role>,
    created_at: DateTime<Utc>,
    last_login: Option<DateTime<Utc>>,
    mfa_enabled: bool,
    mfa_secret: Option<String>,
}

pub enum Role {
    Admin,          // Full access
    Operator,       // View + control
    Viewer,         // Read-only
    PluginManager,  // Plugin management only
}

pub struct Permission {
    resource: Resource,
    action: Action,
}

pub enum Resource {
    Miniserver,
    MqttGateway,
    Plugins,
    Settings,
    Logs,
    Backup,
}

pub enum Action {
    Read,
    Write,
    Delete,
    Execute,
}
```

#### Authentication Methods

**Files to create:**
- `crates/auth/src/lib.rs` - Authentication system
- `crates/auth/src/jwt.rs` - JWT token generation/validation
- `crates/auth/src/session.rs` - Session management
- `crates/auth/src/mfa.rs` - Multi-factor authentication
- `crates/auth/src/oauth.rs` - OAuth2 integration (Google, GitHub)

**Features**:
- ✅ JWT-based authentication
- ✅ Session management with Redis (optional)
- ✅ Password hashing with Argon2
- ✅ Multi-factor authentication (TOTP)
- ✅ OAuth2 login (Google, GitHub, Azure AD)
- ✅ API key management
- ✅ Rate limiting per user
- ✅ Account lockout after failed attempts

#### API Key System

```rust
pub struct ApiKey {
    id: Uuid,
    key: String,           // SHA-256 hashed
    name: String,
    user_id: Uuid,
    permissions: Vec<Permission>,
    expires_at: Option<DateTime<Utc>>,
    last_used: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}
```

**Usage**:
```bash
# Create API key
curl -X POST https://loxberry/api/auth/keys \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{"name":"Plugin API","permissions":["plugins:read","plugins:write"]}'

# Use API key
curl -H "X-API-Key: lbx_abc123..." https://loxberry/api/plugins
```

### 1.2 TLS/SSL Configuration

**Features**:
- ✅ HTTPS-only mode
- ✅ Let's Encrypt automatic certificates
- ✅ Custom certificate upload
- ✅ Certificate auto-renewal
- ✅ HTTP → HTTPS redirect
- ✅ HSTS header

**Implementation**:
```toml
[dependencies]
rustls = "0.21"
tokio-rustls = "0.24"
acme-lib = "0.9"  # Let's Encrypt
```

**Configuration** (`config/system/tls.json`):
```json
{
  "enabled": true,
  "port": 443,
  "certificate_source": "letsencrypt",
  "domains": ["loxberry.local", "loxberry.example.com"],
  "letsencrypt_email": "admin@example.com",
  "auto_renew": true,
  "hsts_enabled": true,
  "min_tls_version": "1.2"
}
```

### 1.3 Security Scanning

**Tools**:
- `cargo audit` - Dependency vulnerability scanning
- `cargo clippy` - Linting for security issues
- `cargo deny` - License and security checks
- OWASP ZAP - Web application security testing

**CI/CD Integration**:
```yaml
# .github/workflows/security.yml
security-scan:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Security Audit
      run: cargo audit
    - name: License Check
      run: cargo deny check
    - name: OWASP ZAP
      uses: zaproxy/action-baseline@v0.7.0
```

### 1.4 Input Validation & Sanitization

**Create validation crate:**
- `crates/validation/src/lib.rs` - Input validation
- `crates/validation/src/sanitize.rs` - Input sanitization

```rust
pub trait Validate {
    fn validate(&self) -> Result<(), ValidationError>;
}

impl Validate for MiniserverConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate IP address format
        IpAddr::from_str(&self.ipaddress)
            .map_err(|_| ValidationError::InvalidIpAddress)?;

        // Validate port range
        if self.port < 1 || self.port > 65535 {
            return Err(ValidationError::InvalidPort);
        }

        // Sanitize credentials (no SQL injection)
        sanitize_string(&self.admin)?;

        Ok(())
    }
}
```

**Common Vulnerabilities to Prevent**:
- SQL Injection (use parameterized queries)
- XSS (escape HTML in templates)
- CSRF (add CSRF tokens)
- Command Injection (validate shell inputs)
- Path Traversal (validate file paths)
- SSRF (validate external URLs)

### 1.5 Security Headers

**HTTP Headers**:
```rust
// Add to all responses
response.headers_mut().insert(
    "X-Content-Type-Options", "nosniff"
);
response.headers_mut().insert(
    "X-Frame-Options", "SAMEORIGIN"
);
response.headers_mut().insert(
    "X-XSS-Protection", "1; mode=block"
);
response.headers_mut().insert(
    "Content-Security-Policy",
    "default-src 'self'; script-src 'self' 'unsafe-inline'"
);
response.headers_mut().insert(
    "Strict-Transport-Security",
    "max-age=31536000; includeSubDomains"
);
```

### 1.6 Audit Logging

**Log all sensitive operations**:

```rust
pub struct AuditLog {
    timestamp: DateTime<Utc>,
    user_id: Uuid,
    action: AuditAction,
    resource: String,
    ip_address: IpAddr,
    user_agent: String,
    success: bool,
    details: serde_json::Value,
}

pub enum AuditAction {
    Login,
    Logout,
    CreateUser,
    DeleteUser,
    UpdateConfig,
    InstallPlugin,
    UninstallPlugin,
    CreateBackup,
    RestoreBackup,
    SendCommand,
}
```

**Storage**: Write to separate audit log file (append-only)

## 2. High Availability & Scalability (Priority: HIGH)

### 2.1 Database Layer

**Current**: JSON files
**Target**: Optional PostgreSQL/SQLite for scalability

**Files to create:**
- `crates/database/src/lib.rs` - Database abstraction
- `crates/database/src/postgres.rs` - PostgreSQL implementation
- `crates/database/src/sqlite.rs` - SQLite implementation
- `crates/database/src/json.rs` - JSON file implementation (backward compat)

```rust
#[async_trait]
pub trait Database: Send + Sync {
    async fn get_config(&self) -> Result<Config>;
    async fn save_config(&self, config: &Config) -> Result<()>;
    async fn get_plugins(&self) -> Result<Vec<PluginEntry>>;
    async fn get_plugin(&self, md5: &str) -> Result<Option<PluginEntry>>;
    async fn save_plugin(&self, plugin: &PluginEntry) -> Result<()>;
    async fn delete_plugin(&self, md5: &str) -> Result<()>;
}

pub enum DatabaseBackend {
    Json,       // Current implementation
    Sqlite,     // Embedded database
    Postgres,   // Production database
}
```

**Migration Path**:
1. Implement database trait
2. Keep JSON as default
3. Add migration tool
4. Support multiple backends

### 2.2 Load Balancing

**Setup**:
```yaml
# docker-compose-ha.yml
version: '3.8'
services:
  loxberry-1:
    image: ghcr.io/boernmaster/rustylox:latest
    # ...

  loxberry-2:
    image: ghcr.io/boernmaster/rustylox:latest
    # ...

  nginx:
    image: nginx:alpine
    ports:
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
    depends_on:
      - loxberry-1
      - loxberry-2
```

**nginx.conf**:
```nginx
upstream loxberry {
    least_conn;
    server loxberry-1:8080;
    server loxberry-2:8080;
}

server {
    listen 443 ssl http2;
    server_name loxberry.example.com;

    ssl_certificate /etc/ssl/cert.pem;
    ssl_certificate_key /etc/ssl/key.pem;

    location / {
        proxy_pass http://loxberry;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

### 2.3 Redis for Session Storage

**Current**: In-memory sessions (lost on restart)
**Target**: Redis-backed sessions

```toml
[dependencies]
redis = { version = "0.23", features = ["tokio-comp", "connection-manager"] }
```

```rust
pub struct RedisSessionStore {
    client: redis::Client,
}

impl RedisSessionStore {
    pub async fn get_session(&self, session_id: &str) -> Result<Option<Session>> {
        let mut conn = self.client.get_async_connection().await?;
        let data: Option<String> = conn.get(format!("session:{}", session_id)).await?;
        Ok(data.and_then(|d| serde_json::from_str(&d).ok()))
    }

    pub async fn save_session(&self, session: &Session) -> Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        let data = serde_json::to_string(session)?;
        conn.set_ex(
            format!("session:{}", session.id),
            data,
            3600  // 1 hour TTL
        ).await?;
        Ok(())
    }
}
```

### 2.4 Message Queue (Optional)

**For background jobs**:

```toml
[dependencies]
lapin = "2.3"  # RabbitMQ client
```

**Use Cases**:
- Plugin installation (long-running)
- Backup creation
- Update downloads
- Email sending
- Log aggregation

```rust
pub struct JobQueue {
    connection: lapin::Connection,
    channel: lapin::Channel,
}

pub enum Job {
    InstallPlugin { zip_path: PathBuf },
    CreateBackup { include_plugins: bool },
    SendEmail { to: String, subject: String, body: String },
}

impl JobQueue {
    pub async fn enqueue(&self, job: Job) -> Result<()> {
        let payload = serde_json::to_vec(&job)?;
        self.channel.basic_publish(
            "",
            "jobs",
            BasicPublishOptions::default(),
            &payload,
            BasicProperties::default(),
        ).await?;
        Ok(())
    }
}
```

## 3. Performance Optimization (Priority: MEDIUM)

### 3.1 Caching Layer

**Implement Redis cache**:

```rust
pub struct Cache {
    redis: redis::Client,
    ttl: Duration,
}

impl Cache {
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>>
    pub async fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()>
    pub async fn delete(&self, key: &str) -> Result<()>
    pub async fn exists(&self, key: &str) -> Result<bool>
}
```

**Cache Strategy**:
- Plugin list (5 minutes)
- Miniserver status (30 seconds)
- Configuration (1 hour, invalidate on write)
- System metrics (10 seconds)

### 3.2 Database Connection Pooling

```rust
use sqlx::postgres::PgPoolOptions;

let pool = PgPoolOptions::new()
    .max_connections(10)
    .connect("postgres://localhost/loxberry").await?;
```

### 3.3 Async Optimizations

**Parallel Operations**:
```rust
// Send to multiple Miniservers in parallel
let futures = miniservers.iter()
    .map(|ms| ms.send(params.clone()))
    .collect::<Vec<_>>();

let results = futures::future::join_all(futures).await;
```

**Lazy Loading**:
```rust
// Only load plugin details when needed
pub struct PluginListItem {
    md5: String,
    name: String,
    version: String,
    // Heavy fields loaded on demand
}

impl PluginListItem {
    pub async fn load_full(&self) -> Result<PluginEntry> {
        // Load from database
    }
}
```

### 3.4 Static Asset Optimization

- Gzip/Brotli compression
- HTTP caching headers
- CDN integration (optional)
- Asset bundling/minification

```rust
use tower_http::compression::CompressionLayer;
use tower_http::set_header::SetResponseHeaderLayer;

Router::new()
    .layer(CompressionLayer::new())
    .layer(SetResponseHeaderLayer::if_not_present(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    ))
```

## 4. Cloud Deployment Support (Priority: LOW)

### 4.1 Kubernetes Deployment

**Files to create:**
- `k8s/deployment.yaml` - Kubernetes deployment
- `k8s/service.yaml` - Service definition
- `k8s/ingress.yaml` - Ingress controller
- `k8s/configmap.yaml` - Configuration
- `k8s/secret.yaml` - Secrets

**Example Deployment**:
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: loxberry
spec:
  replicas: 3
  selector:
    matchLabels:
      app: loxberry
  template:
    metadata:
      labels:
        app: loxberry
    spec:
      containers:
      - name: loxberry
        image: ghcr.io/boernmaster/rustylox:latest
        ports:
        - containerPort: 8080
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: loxberry-secrets
              key: database-url
        volumeMounts:
        - name: config
          mountPath: /opt/loxberry/config
      volumes:
      - name: config
        persistentVolumeClaim:
          claimName: loxberry-config
```

### 4.2 Cloud Provider Integrations

**AWS**:
- ECS/Fargate deployment
- RDS for database
- S3 for backups
- CloudWatch for logs/metrics

**Azure**:
- Azure Container Instances
- Azure Database for PostgreSQL
- Azure Blob Storage for backups
- Azure Monitor

**Google Cloud**:
- Cloud Run
- Cloud SQL
- Cloud Storage
- Cloud Logging

### 4.3 Terraform Configuration

**Files to create:**
- `terraform/main.tf` - Main configuration
- `terraform/variables.tf` - Variables
- `terraform/outputs.tf` - Outputs
- `terraform/aws.tf` - AWS resources
- `terraform/azure.tf` - Azure resources
- `terraform/gcp.tf` - GCP resources

## 5. Observability & Tracing (Priority: MEDIUM)

### 5.1 Distributed Tracing

**Integration with Jaeger/Zipkin**:

```toml
[dependencies]
opentelemetry = "0.20"
opentelemetry-jaeger = "0.19"
tracing-opentelemetry = "0.21"
```

```rust
use opentelemetry::global;
use tracing_subscriber::layer::SubscriberExt;

let tracer = opentelemetry_jaeger::new_agent_pipeline()
    .with_service_name("loxberry")
    .install_simple()?;

let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

tracing_subscriber::registry()
    .with(telemetry)
    .init();
```

**Trace spans**:
```rust
#[tracing::instrument]
async fn install_plugin(zip_path: PathBuf) -> Result<()> {
    let span = tracing::info_span!("extract_zip");
    let _guard = span.enter();
    // ... operation
}
```

### 5.2 Log Aggregation

**Integration with ELK/Loki**:

- Elasticsearch for log storage
- Logstash for log processing
- Kibana for visualization
- Grafana Loki (lightweight alternative)

### 5.3 APM (Application Performance Monitoring)

**Integration with**:
- Datadog APM
- New Relic
- Elastic APM
- Honeycomb

## Success Criteria

- [ ] HTTPS-only deployment with Let's Encrypt
- [ ] Role-based access control (RBAC)
- [ ] Multi-factor authentication (MFA)
- [ ] API key system operational
- [ ] Security audit passing
- [ ] Load balancing working
- [ ] Database layer abstraction complete
- [ ] Redis caching functional
- [ ] Performance benchmarks met (>1000 req/s)
- [ ] Kubernetes deployment tested
- [ ] Distributed tracing working
- [ ] Production monitoring dashboard
- [ ] Security documentation complete

## Dependencies

```toml
[dependencies]
# Auth & Security
jsonwebtoken = "9.2"
argon2 = "0.5"
totp-rs = "5.0"
rustls = "0.21"
acme-lib = "0.9"

# Database
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-rustls"] }
redis = { version = "0.23", features = ["tokio-comp"] }

# Observability
opentelemetry = "0.20"
opentelemetry-jaeger = "0.19"
tracing-opentelemetry = "0.21"

# Cloud
aws-sdk-s3 = "1.0"
azure_storage = "0.18"
google-cloud-storage = "0.16"
```

## Timeline

**Estimated Duration**: 8-12 weeks

### Weeks 1-2: Security Foundation
- Authentication system
- Authorization/RBAC
- TLS/SSL setup
- Security headers

### Weeks 3-4: Database & Caching
- Database abstraction layer
- Redis integration
- Migration tools
- Performance testing

### Weeks 5-6: High Availability
- Load balancer setup
- Session management
- Multi-instance testing
- Failover testing

### Weeks 7-8: Cloud Integration
- Kubernetes manifests
- Terraform configurations
- Cloud provider testing
- CI/CD for cloud deployments

### Weeks 9-10: Observability
- Distributed tracing
- Log aggregation
- APM integration
- Monitoring dashboards

### Weeks 11-12: Testing & Documentation
- Security audit
- Performance benchmarks
- Production deployment guide
- Disaster recovery procedures

## Next Steps After Phase 7

Future considerations:
- Plugin marketplace
- Multi-tenancy support
- GraphQL API
- WebAssembly plugins
- Mobile app
- Voice assistant integration
