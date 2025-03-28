# TODO

## TODO

*   **Implement Prefix Rule Manager:**
    *   Implement data model for prefix rules (prefix\_key, format, seq\_length, initial\_seq).
    *   Implement registration rule API (`PUT /prefix-configs/{prefixKey}`).
    *   Implement query rule API (`GET /prefix-configs/{prefixKey}`).
    *   Choose storage implementation (Redis Hash / Relational Database).
*   **Implement Sequence Generator:**
    *   Implement concurrency control strategy (Redis atomic operations, Database row lock, Snowflake).
    *   Implement atomic increment verification unit test.
    *   Implement initial sequence processing unit test.
    *   Implement uniqueness across nodes unit test.
    *   Implement sequence overflow handling unit test.
*   **Implement Number Assembler:**
    *   Implement formatting logic to combine prefix and sequence.
*   **Implement Generate Number Interface:**
    *   Implement API endpoint (`GET /api/numbers`).
    *   Implement request parameter validation (prefixKey).
    *   Return formatted number in response.
*   **Implement Prefix Rule Management Interface:**
    *   Implement API endpoint (`PUT /api/prefix-configs/{prefixKey}`).
    *   Implement request body validation (format, seqLength, initialSeq).
    *   Return 200 OK on success.
*   **Implement Error Handling:**
    *   Implement prefix not registered error.
    *   Implement storage layer connection failure error.
    *   Implement input parameter validation error.
    *   Implement error handler unit tests.
*   **Implement E2E Tests:**
    *   Implement Hurl functional tests.
        *   Successful registration and generation.
        *   Error handling process.
        *   Idempotence verification.
        *   Cross-prefix isolation.
        *   Invalid format registration.
    *   Implement k6 performance tests.
        *   Benchmark Load Test.
        *   Peak traffic test.
        *   Endurance test.
        *   Fault recovery test.
    *   Implement Security Testing
        *   Unauthorized access
        *   Input Injection Attack
        *   Sensitive log mask
*   **Implement Fault Tolerance:**
    *   Implement Redis unavailable degradation strategy (local segment cache).
    *   Implement service node downtime handling (K8s restarts).
    *   Implement network partitioning handling (local clock).
*   **Implement Monitoring and Alerting:**
    *   Collect performance indicators (Prometheus).
    *   Monitor numbers and generate health status (Grafana).
*   **Implement Horizontal Scalability:**
    *   Redis Cluster for distributed storage.
    *   Service statelessness for load balancing.
