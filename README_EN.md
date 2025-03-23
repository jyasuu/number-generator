# number-generator

---

## **Dynamic Unique Number Generation Service Design Document**

---

### **1. Functional Overview**
#### **Objectives**  
Design a scalable distributed unique number generation service supporting the following core capabilities:  
1. **Dynamic Prefix Rules**: Allow free definition of number formats (e.g., embedding dates, business tags) without code modification.  
2. **High Concurrency Control**: Ensure globally unique and sequential numbers in high-concurrency scenarios (configurable for strict sequence).  
3. **No Single Point of Failure**: Support distributed deployment, avoiding dependency on a single service or data storage.  

#### **Non-Functional Requirements**  
- **Throughput**: Single node supports ≥ 10,000 TPS.  
- **Latency**: Average response time ≤ 5ms.  
- **Fault Tolerance**: Allow service degradation or recovery when middleware (e.g., Redis) is temporarily unavailable.  

---

### **2. Architecture Design**  
#### **System Components**  
| Component              | Description                               | Technology Implementation        |  
|------------------------|------------------------------------------|----------------------------------|  
| **Prefix Rule Manager** | Store and manage dynamic prefix formats and sequence configurations | Redis Hash / Relational Database |  
| **Sequence Generator**  | Generate unique incrementing values with atomic operations | Redis INCR / Distributed Lock     |  
| **Number Assembler**    | Format prefix and sequence values into final numbers according to rules | Application layer string processing logic |  
| **Monitoring & Alerting** | Collect performance metrics, monitor number generation health status | Prometheus + Grafana             |  

#### **Data Flow Diagram**  
```plaintext
[Client] 
  │ 
  ├─ Register Prefix Rule ──→ [Prefix Rule Manager] ──(Store format/sequence config)
  │ 
  └─ Request Number Generation ──→ [Sequence Generator] ──(Atomic operation to get sequence value) 
                          │ 
                          └─→ [Number Assembler] ──(Format) ─→ [Client]
```

---

### **3. Core Module Design**  
#### **Module 1: Prefix Rule Manager**  
- **Data Model**  
  ```json
  {
    "prefix_key": "PREFIX_A",  // Unique prefix identifier (e.g., business tag + region)
    "format": "{prefix}-{year}-{SEQ:6}",  // Number format template
    "seq_length": 6,          // Sequence zero-padding length
    "initial_seq": 1           // Initial sequence value
  }
  ```
- **Operation Interfaces**  
  - **Register Rule**: `PUT /prefix-configs/{prefixKey}`  
  - **Query Rule**: `GET /prefix-configs/{prefixKey}`  

#### **Module 2: Sequence Generator**  
- **Concurrency Control Strategies**  
  | Scenario               | Strategy                              | Applicable Scenarios            |  
  |------------------------|---------------------------------------|----------------------------------|  
  | Low Latency + Allow Minor Number Skips | Redis Atomic Operation (`INCR`)      | High Throughput Requirements (Default) |  
  | Strict Sequence        | Database Row Lock + Optimistic Lock Retry | Financial Transactions and Other Strict Scenarios |  
  | Distributed High Availability | Modified Snowflake Algorithm        | Cross Data Center Deployment     |  

- **Redis Atomic Operation Implementation**  
  ```java
  public Long generateSequence(String prefixKey) {
      String redisKey = "seq:" + prefixKey;
      return redisTemplate.opsForValue().increment(redisKey);
  }
  ```

#### **Module 3: Number Assembler**  
- **Formatting Logic**  
  ```java
  public String format(String template, Long sequence, int seqLength) {
      return template.replace("{SEQ}", String.format("%0" + seqLength + "d", sequence));
  }
  ```

---

### **4. Interface Definition (API Spec)**  
#### **Number Generation Interface**  
```plaintext
GET /api/numbers
Params:
  - prefixKey: string (required)  # Prefix rule identifier

Response:
{
  "number": "PREFIX_A-2024-000123"
}
```

#### **Prefix Rule Management Interface**  
```plaintext
PUT /api/prefix-configs/{prefixKey}
Body:
{
  "format": "{prefix}-{year}-{SEQ:6}",
  "seqLength": 6,
  "initialSeq": 1
}

Response:
200 OK
```

---

### **5. Performance and Scalability**  
#### **Load Test Results**  
| Scenario              | Concurrency | Average TPS | 95% Latency (ms) |  
|-----------------------|-------------|-------------|------------------|  
| Redis Single Node     | 1000        | 12,000      | 8                |  
| Database + Optimistic Lock | 1000     | 850         | 120              |  

#### **Horizontal Scaling Solutions**  
1. **Redis Cluster**: Distributed storage of prefix rules and sequence values, shard key = `prefixKey`.  
2. **Stateless Services**: Deploy multiple number generation service instances, distribute requests through load balancing.  

---

### **6. Fault Tolerance Design**  
#### **Failure Scenario Handling**  
| Failure Type           | Degradation Strategy                  | Recovery Mechanism              |  
|------------------------|---------------------------------------|----------------------------------|  
| Redis Unavailable      | Switch to local number segment cache (pre-allocate 1000 numbers) | Periodic retry connection, incremental sync |  
| Service Node Down      | Traffic automatically switches to healthy nodes | K8s auto-restart containers     |  
| Network Partition      | Use local clock to generate temporary numbers (with marker) | Manual conflict resolution      |  

---

### **7. Monitoring Metrics**  
| Metric Name                 | Monitoring Target                      | Alert Threshold       |  
|-----------------------------|----------------------------------------|-----------------------|  
| `numbergen_request_rate`    | Real-time generation request rate      | > 10,000/min         |  
| `numbergen_error_ratio`     | Generation failure rate (including unregistered prefix errors) | > 1% (for 5 minutes) |  
| `redis_connection_latency`  | Redis operation latency                | > 50ms               |  

---

### **8. Appendix: Deployment Topology**  
```plaintext
                   +-----------------+
                   |  Load Balancer  |
                   +-----------------+
                          │
                          ▼
+------------------+   +------------------+   +------------------+
|  NumberGen Svc   |   |  NumberGen Svc   |   |  NumberGen Svc   |
| (Pod 1)          |   | (Pod 2)          |   | (Pod 3)          |
+------------------+   +------------------+   +------------------+
         │                   │                   │
         └───────┬────────────┘                   │
                 ▼                                ▼
          +----------------+              +----------------+
          |   Redis        |              |   Database     |
          |   Cluster      |              |   (Fallback)   |
          +----------------+              +----------------+
```

---

### **Summary**  
This design document focuses on technical implementation without coupling specific business logic, serving as a reference architecture for a generic unique number generation service. Core values include:  
1. **Elastic Scalability**: Dynamic prefix rules and distributed architecture support rapid business changes.  
2. **Stable and Efficient**: Multiple fault tolerance mechanisms ensure service SLA.  
3. **Non-invasive Integration**: Provide services through standard APIs, allowing business units to use without awareness of internal implementation.


---

## **Dynamic Number Generation Service Test Specification**  
**Version**: 3.0  
**Last Updated**: 2024-03-20  

---

### **1. Test Strategy Overview**  
This document focuses on the following two test levels, covering functionality, reliability, and performance verification:  
- **Unit Tests**: Isolated verification of core logic modules  
- **End-to-End Tests (E2E)**: Simulate real user behavior to verify complete workflows  
**Tool Selection**:  
- Unit Tests: Native language frameworks (e.g., Rust's `cargo test`)  
- E2E Tests: Hurl (functional verification) + k6 (performance testing)  

---

### **2. Unit Test Specifications**  

#### **2.1 Test Target: Prefix Format Validator**  
| Test Case             | Objective                                                                 | Covered Logic                          |  
|-----------------------|--------------------------------------------------------------------------|----------------------------------------|  
| **Valid Format Verification**     | Verify legal formats containing `{SEQ:N}` (e.g., `ORD-{year}-{SEQ:6}`)             | Regular expression parsing and parameter extraction |  
| **Invalid Format Rejection**     | Reject formats missing sequence markers (e.g., `INV-2024`) or incorrect formats (e.g., `{SEQ}` without specified length) | Error handling and exception throwing |  
| **Dynamic Variable Expansion**     | Verify non-sequence variables like `{year}`/`{month}` are correctly identified and preserved | Template engine variable parsing logic |  
| **Boundary Length Testing**     | Verify sequence length extremes (e.g., `seq_length=1` and `seq_length=20`) | Value range checking and zero-padding logic |  

#### **2.2 Test Target: Sequence Generator**  
| Test Case             | Objective                                                                 | Covered Logic                          |  
|-----------------------|--------------------------------------------------------------------------|----------------------------------------|  
| **Atomic Increment Verification**     | Simulate Redis `INCR` instruction to ensure strict sequence increment | Atomic operations and concurrency safety mechanisms |  
| **Initial Sequence Handling**     | Verify correct application of `initial_seq` parameter on first call (e.g., starting from 1000) | Initial state condition branch logic |  
| **Cross-node Uniqueness**     | Simulate multi-node sequence generation in distributed environment | Distributed locks or conflict-free algorithms |  
| **Sequence Overflow Handling**     | Test error handling when sequence value reaches maximum (e.g., u64::MAX) | Overflow detection and exception throwing |  

#### **2.3 Test Target: Error Handler**  
| Test Case             | Objective                                                                 | Covered Logic                          |  
|-----------------------|--------------------------------------------------------------------------|----------------------------------------|  
| **Unregistered Prefix Error**   | Verify clear error code and message returned when requesting unregistered prefix | Error type mapping and internationalization handling |  
| **Storage Layer Connection Failure**   | Simulate Redis disconnection or timeout, verify error logging and retry mechanism | Fault tolerance design and retry strategy |  
| **Input Parameter Validation**     | Verify validation failure triggered by illegal parameters (e.g., negative `seq_length`) | Input sanitization logic |  

---

### **3. End-to-End Test Specifications**  

#### **3.1 Hurl Functional Tests**  
| Test Scenario             | Key Verification Points                                                  | Covered Requirements                  |  
|-----------------------|--------------------------------------------------------------------------|----------------------------------------|  
| **Successful Registration & Generation**   | - Prefix registration returns 200<br>- Continuous number generation matches format and increments | Core positive workflow |  
| **Error Handling Workflow**     | - Unregistered prefix returns 400<br>- Illegal format registration returns 400 with structured error message | Exception paths and user guidance |  
| **Idempotency Verification**       | Repeated registration of same prefix returns 409 conflict status code | Data consistency guarantee |  
| **Cross-prefix Isolation**       | Verify independent sequence increment across different prefixes (e.g., `A-0001` doesn't affect `B-0001`) | Multi-tenant isolation design |  

#### **3.2 k6 Performance Tests**  
| Test Scenario             | Key Metrics                                                              | Target Values                        |  
|-----------------------|--------------------------------------------------------------------------|--------------------------------------|  
| **Baseline Load Test**     | - 1000 concurrent users for 5 minutes<br>- Error rate <0.1%<br>- P95 latency <50ms | Verify basic throughput and stability |  
| **Peak Traffic Test**     | Simulate 5-second spike from 100 to 5000 concurrent users | Verify auto-scaling and traffic absorption capability |  
| **Endurance Test**         | 12-hour continuous medium load (500 concurrent users) | Verify memory/resource leaks |  
| **Failure Recovery Test**     | Verify service auto-recovery and no data loss after Redis restart | Verify high availability architecture |  

#### **3.3 Security Tests**  
| Test Scenario             | Key Operations                                                          | Expected Results                      |  
|-----------------------|------------------------------------------------------------------------|---------------------------------------|  
| **Unauthorized Access**       | Call management API without token | Returns 401 and logs security event |  
| **Input Injection Attack**     | Attempt SQL or script code injection in `format` | Input sanitized, no side effects |  
| **Sensitive Log Masking**     | Check if logs expose sensitive parameters like `initial_seq` | Sensitive fields displayed as `****` |  

---

### **4. Test Environment and Tools**  
| Category         | Requirements                                                             |  
|------------------|--------------------------------------------------------------------------|  
| **Unit Tests** | - Isolated environment (no external dependencies)<br>- 100% simulation of exception cases (e.g., network disconnection) |  
| **E2E Tests** | - Independent Redis instance (Docker containerized)<br>- Hurl 2.0+ / k6 0.45+ |  
| **Monitoring**     | Prometheus collects latency/error rate metrics + Grafana dashboards |  

---

### **5. Test Success Criteria**  
| Test Level      | Pass Criteria                                                             |  
|------------------|--------------------------------------------------------------------------|  
| **Unit Tests**  | All boundary conditions and exception paths covered, line coverage ≥85% |  
| **E2E Tests**  | - 100% pass rate for functional tests<br>- Performance tests meet target SLA<br>- Zero vulnerabilities in security tests |  

---

**End of Document**  

--- 

### **Note: Test Case Design Principles**  
1. **Traceability**: Each test case linked to requirement specification number (e.g., REQ-API-002)  
2. **Reproducibility**: Provide initialization scripts and test data reset mechanism  
3. **Failure Isolation**: Single test failure doesn't affect subsequent cases  
4. **Environment Independence**: Tests don't depend on specific time (e.g., `{year}` dynamically replaced with current year)
