# Changelog

## 0.1.0

- Implemented dynamic prefix rules with Redis-backed PrefixRuleManager.
- Implemented efficient concurrency control using Redis atomic INCR operation.
- Implemented service statelessness for horizontal scaling and no single point of failure.
- Implemented Prefix rule management interface (PUT /api/prefix-configs/{prefixKey}, GET /api/prefix-configs/{prefixKey}).
- Implemented Generate Number Interface (GET /api/numbers).
- Implemented Redis Cluster support for horizontal expansion.
- Implemented Redis unavailability fault tolerance mechanism with local cache and retry logic.
- Implemented service node downtime fault tolerance mechanism.
- Implemented network partitioning fault tolerance mechanism with local clock and temporary number generation.
- Added API endpoint to manually set network partition status for a prefix rule.
- Implemented Hurl functional tests for generate number, idempotent registration, invalid format registration, and cross-prefix isolation.
