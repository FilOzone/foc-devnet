# YugabyteDB Port Reference

## Required Ports for foc-yugabyte Docker Container

The following ports need to be exposed when running the YugabyteDB container:

```
-p 5433:5433    # YSQL (PostgreSQL-compatible API)
-p 9042:9042    # YCQL (Cassandra-compatible API)
-p 7000:7000    # YB-Master RPC
-p 9000:9000    # YB-Master Admin UI
-p 7100:7100    # YB-TServer RPC
-p 9100:9100    # YB-TServer Admin UI
-p 15433:15433  # YugabyteDB Web UI (Main Dashboard)
```

## Quick Start

```bash
docker run -d \
  --name foc-yugabyte \
  -p 5433:5433 \
  -p 9042:9042 \
  -p 7000:7000 \
  -p 9000:9000 \
  -p 7100:7100 \
  -p 9100:9100 \
  -p 15433:15433 \
  foc-yugabyte
```

## Access Points

- **Database Connection**: `localhost:5433` (YSQL/PostgreSQL)
- **Web Dashboard**: http://localhost:15433
- **Master Admin**: http://localhost:9000
- **TServer Admin**: http://localhost:9100
