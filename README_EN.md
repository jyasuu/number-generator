# number-generator

---

## **Dynamic unique number generation service design file**

---

### **1. Functional Overview**
#### **Target**  
Design a scalable distributed unique number generation service that supports the following core capabilities:  
1. **Dynamic prefix rules**: Allows you to freely define numbering formats (such as embedding dates, business labels) without modifying the code.  
2. **Efficient concurrency control**: In high-concurrency scenarios, ensure that the numbers are globally unique and continuous (you can configure whether they are strictly continuous).  
3. **No single point of failure**: Supports distributed deployment to avoid dependence on a single service or data storage.  

#### **Non-functional requirements**  
- **Throughput**: Single node supports ≥ 10,000 TPS.  
- **Latency**: Average response time ≤ 5ms.  
- **Fault tolerance**: Allows the service to automatically downgrade or recover when the middleware (such as Redis) is temporarily unavailable.  

---

### **2. Architecture Design**  
#### **System Components**  
| Components | Responsibilities | Technology Implementation Selection |  
|----------------------|----------------------------------------|--------------------------|  
| **Prefix Rule Manager** | Store and manage dynamic prefix formats and sequence configurations | Redis Hash / Relational Database |  
| **Sequence Generator** | Generate unique incrementing values, support atomic operations | Redis INCR / Distributed Lock |  
| **Number Assembler** | Format the prefix and sequence value as the final number according to the rules | Application layer string processing logic |  
| **Monitoring and Alerting** | Collect performance indicators, monitor numbers and generate health status | Prometheus + Grafana |  

#### **Data Flow Diagram**  
```Plain Text
[Client]
  │
  ├─ Register prefix rules ──→ [Prefix rule manager] ──(Storage format/sequence configuration)
  │
  └─ Request to generate a number ──→ [Sequence Generator] ──(Atomic operation to obtain sequence value)
                          │
                          └─→ [Number Assembler] ──(Formatter) ─→ [Client]
```

---

### **3. Core module design**  
#### **Module 1: Prefix Rule Manager**  
- **Data Model**  
  ```json
  {
    "prefix_key": "PREFIX_A", // Unique prefix identifier (such as business label + region)
    "format": "{prefix}-{year}-{SEQ:6}", // Numbering format template
    "seq_length": 6, // sequence zero-filling length
    "initial_seq": 1 // Initial sequence value
  }
  ```
- **Operation Interface**  
  - **Registration rules**: `PUT /prefix-configs/{prefixKey}`  
  - **Query rule**: `GET /prefix-configs/{prefixKey}`  

#### **Module 2: Sequence Generator**  
- **Concurrency Control Strategy**  
  | Scenario| Strategy| Applicable Scenarios|  
  |----------------------|-----------------------------------|-------------------------|  
  | Low latency + allow for slight number skipping | Redis atomic operations (`INCR`) | High throughput requirements (preset solution) |  
  | Strict continuity | Database row lock + optimistic lock retry | Rigorous scenarios such as financial transactions |  
  | Distributed high availability | Based on Snowflake algorithm transformation | Deployment across data centers |  

---

### **4. Interface Definition (API Spec)**  
#### **Generate Number Interface**  
```Plain Text
GET /api/numbers
Params:
  - prefixKey: string (required) # prefix rule identifier

Response:
{
  "number": "PREFIX_A-2024-000123"
}
```

#### **Prefix rule management interface**  
```Plain Text
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
| Scenario | Concurrency | Average TPS | 95% Latency (ms) |  
|---------------------|--------|----------|---------------|  
| Redis Single Node | 1000 | 12,000 | 8 |  
| Database + Optimistic Locking | 1000 | 850 | 120 |  

#### **Horizontal expansion plan**  
1. **Redis Cluster**: Distributed storage of prefix rules and sequence values, shard key = `prefixKey`.  
2. **Service statelessness**: deploy multiple number-taking service instances and distribute requests through load balancing.  

---

### **6. Fault-tolerant design**  
#### **Fault Scenario Handling**  
| Failure Type| Degradation Strategy| Recovery Mechanism|  
|----------------------|-----------------------------------|--------------------------|  
| Redis is unavailable | Switch to local segment cache (pre-allocate 1000) | Retry the connection periodically, incremental synchronization |  
| Service node downtime | Traffic automatically switches to healthy nodes | K8s automatically restarts containers |  
| Network partitioning | Using local clock to generate temporary numbers (including tags) | Manual intervention conflict resolution |  

---

### **8. Appendix: Deployment topology diagram**  
```Plain Text
                   +-----------------+
                   | Load Balancer |
                   +-----------------+
                          │
                          ▼
+------------------+ +------------------+ +------------------+
| NumberGen Svc | | NumberGen Svc | | NumberGen Svc |
| (Pod 1) | | (Pod 2) | | (Pod 3) |
+------------------+ +------------------+ +------------------+
         │ │ │
         └───────┬────────────┘ │
                 ▼ ▼
          +----------------+ +----------------+
          | Redis | | Database |
          | Cluster | | (Fallback) |
          +----------------+ +----------------+
```

---

### **Summarize**  
This design document focuses on technical implementation and is not coupled with specific business logic. It can be used as an architectural reference for universal unique number generation services. The core values ​​are:  
1. **Elastic scalability**: Dynamic prefix rules and distributed architecture support rapid business changes.  
2. **Stable and efficient**: Multiple fault-tolerant mechanisms guarantee service SLA.  
3. **Non-intrusive integration**: Services are provided through standard APIs, and the business side does not need to be aware of the internal implementation.


---


### **1. Overview of testing strategy**  
This document focuses on the following two test levels, covering functionality, reliability and performance verification:  
- **Unit testing**: Isolate and verify the core logic module  
- **End-to-end testing (E2E): simulate real user behavior to verify the complete process  
**Tool Selection**:  
- Unit testing: language-native frameworks (such as Rust's `cargo test`)  
- E2E test: Hurl (functional verification) + k6 (performance stress test)  

---

### **2. Unit Test Specification**  

#### **2.1 Test object: prefix format validator**  
| Test Cases | Goals | Coverage Logic |  
|----------------------|----------------------------------------------------------------------|----------------------------------|  
| **Validation of legal format** | Validation of legal format containing `{SEQ:N}` (such as `ORD-{year}-{SEQ:6}`) | Regular expression parsing and parameter extraction |  
| **Illegal format rejection** | Reject missing sequence markers (such as `INV-2024`) or malformed formats (such as `{SEQ}` with unspecified length) | Error handling flow and exception throwing |  
| **Dynamic variable expansion** | Verify that non-sequence variables such as `{year}`/`{month}` are correctly identified and retained | Template engine variable parsing logic |  
| **Boundary length test** | Verify sequence length extremes (such as `seq_length=1` and `seq_length=20`) | Numeric range check and zero padding logic |  

#### **2.2 Test object: sequence generator**  
| Test Cases | Goals | Coverage Logic |  
|----------------------|----------------------------------------------------------------------|----------------------------------|  
| **Atomic increment verification** | Simulate Redis `INCR` instruction to ensure strict sequence increment | Atomic operation and concurrent safety mechanism |  
| **Initial sequence processing** | Verify that `initial_seq` parameter is applied correctly on first call (e.g. start incrementing from 1000) | Initial state conditional branching logic |  
| **Uniqueness across nodes** | Simulating multiple nodes generating sequences simultaneously in a distributed environment | Distributed lock or conflict-free algorithm |  
| **Sequence overflow handling** | Error handling when the test sequence value reaches the maximum value (such as u64::MAX) | Overflow detection and exception throwing |  

#### **2.3 Test object: error handler**  
| Test Cases | Goals | Coverage Logic |  
|----------------------|----------------------------------------------------------------------|----------------------------------|  
| **Prefix not registered error** | Return clear error code and message when verification request does not have a registered prefix | Error type mapping and internationalization processing |  
| **Storage layer connection failure** | Simulate Redis disconnection or timeout, verify error log and retry mechanism | Fault-tolerant design and retry strategy |  
| **Input parameter validation** | Validating illegal parameters (such as negative `seq_length`) triggers validation failure | Input Sanitization logic |  

---

### **3. End-to-end test specifications**  

#### **3.1 Hurl functional test**  
| Test scenarios | Key verification points | Coverage requirements |  
|----------------------|----------------------------------------------------------------------|----------------------------------|  
| **Successful registration and generation** | - Registration prefix returns 200 
- Continuous generation numbers conform to the format and increase | Core forward process |  
| **Error handling process** | - Unregistered prefix returns 400 
- Illegal format registration returns 400 and structured error message | Exception path and user guidance |  
| **Idempotence verification** | Repeated registration of the same prefix returns a 409 conflict status code | Data consistency guarantee |  
| **Cross-prefix isolation** | Verify that sequences of different prefixes are incremented independently (e.g. `A-0001` does not affect `B-0001`) | Multi-tenant isolation design |  

#### **3.2 k6 performance test**  
| Test scenario | Key indicators | Target value |  
|----------------------|----------------------------------------------------------------------|---------------------------------|  
| **Benchmark Load Test** | - 1000 concurrent users for 5 minutes 
- Error rate <0.1% 
- P95 latency <50ms | Verify basic throughput and stability |  
| **Peak traffic test** | Simulate a sudden increase from 100 concurrent users to 5000 concurrent users within 5 seconds | Verify automatic expansion and traffic absorption capabilities |  
| **Endurance test** | 12 hours of sustained medium load (500 concurrent) | Verify no memory/resource leaks |  
| **Fault recovery test** | After restarting Redis, verify that the service automatically recovers and no data is lost | Verify high availability architecture |  

#### **3.3 Security Testing**  
| Test Scenario | Key Operations | Expected Results |  
|----------------------|--------------------------------------------------------------------|---------------------------------|  
| **Unauthorized access** | Calling the management API without a token | Return 401 and log security events |  
| **Input Injection Attack** | Try to inject SQL or script code in `format` | Input is sanitized, no side effects |  
| **Sensitive log mask** | Check whether sensitive parameters such as `initial_seq` are exposed in the log | Sensitive fields are displayed as `****` |  

---

### **4. Test environment and tools**  
| Category | Requirements |  
|--------------|---------------------------------------------------------------------|  
| **Unit testing** | - Isolated environment (no external dependencies) 
- 100% simulation of abnormal cases (such as network disconnection) |  
| **E2E Testing** | - Standalone Redis instance (Docker containerized) 
- Hurl 2.0+ / k6 0.45+ |  
| **Monitoring** | Prometheus to collect latency/error rate metrics + Grafana dashboards |  

---

### **5. Test success criteria**  
| Test Level | Pass Condition |  
|--------------|-------------------------------------------------------------------------|  
| **Unit test** | All boundary conditions and exception paths are covered, line coverage ≥ 85% |  
| **E2E testing** | - Functional testing 100% passed 
- Performance testing achieved target SLA 
- Security testing zero vulnerabilities |  

---


### **1. Unit test specifications**  

#### **1.1 Test object: prefix format validator**  
| Test case | Initial data | Operation steps | Assertion data |  
|----------------------|------------------------------------------|----------------------------------|----------------------------------|  
| **Validate the legal format** | `format = "ORD-{year}-{SEQ:6}"` | Parse the format and extract parameters | `seq_length=6`, identify `{year}` |  
| **Illegal format (no SEQ)** | `format = "INVALID-2024"` | Parse format | Throws `InvalidFormatError` |  
| **Illegal length parameter** | `format = "ERR-{SEQ:0}"` | Parsing format | Throws `InvalidLengthError` |  

#### **1.2 Test object: sequence generator**  
| Test case | Initial data | Operation steps | Assertion data |  
|----------------------|------------------------------------------|----------------------------------|----------------------------------|  
| **Atomic increment (single shot)** | `initial_seq=1000` | calls `generate()` | returns `1001` |  
| **Atomic increment (multiple times)** | `initial_seq=1` | Call `generate()` 3 times in a row | Returns `2`, `3`, `4` |  
| **Distributed uniqueness** | Simulate 2 nodes calling `generate()` at the same time | Concurrent requests | Generated values ​​are non-duplicate and continuous |  
| **SEQUENCE OVERFLOW** | `current_seq=18446744073709551615` (u64 MAX) | calls `generate()` | throws `SequenceOverflowError` |  

#### **1.3 Test object: error handler**  
| Test case | Initial data | Operation steps | Assertion data |  
|----------------------|------------------------------------------|----------------------------------|----------------------------------|  
| **Prefix not registered** | Unregistered prefix `UNKNOWN` | Calls `generate("UNKNOWN")` | Throws `PrefixNotRegisteredError` |  
| **Storage layer timeout** | Simulate Redis response timeout (>5s) | Call `generate()` | Throw `StorageTimeoutError` |  
| **Illegal parameter input** | `seq_length = -5` | Register prefix | Throw `ValidationError` |  

---

### **2. End-to-end test specifications**  

#### **2.1 Functional Test (Hurl)**  
| Test scenario | Initial data | Operation steps | Assertion data |  
|----------------------|------------------------------------------|----------------------------------|----------------------------------|  
| **Generate numbers successfully** | Register prefix `TEST-{SEQ:4}` | 1. `POST /prefix-configs/TEST` 
2. `GET /numbers/TEST` 3 times | 1. HTTP 200 
2. Return `TEST-0001`, `TEST-0002`, `TEST-0003` |  
| **Idempotent registration** | Prefix already exists`DUPLICATE` | Repeat `POST /prefix-configs/DUPLICATE` | HTTP 409 + error message `Prefix already exists` |  
| **Isolate across prefixes** | Register `A-{SEQ:3}` and `B-{SEQ:3}` | Alternate calls to `GET /numbers/A` and `GET /numbers/B` | `A-001`, `B-001`, `A-002`, `B-002` |  
| **Invalid format registration** | `format = "INVALID"` | `POST /prefix-configs/INVALID` | HTTP 400 + error message `Missing {SEQ} tag` |  

#### **2.2 Performance Test (k6)**  
| Test scenario | Initial data | Operation steps | Assertion data (SLA) |  
|----------------------|------------------------------------------|----------------------------------|----------------------------------|  
| **Benchmark Load** | Register prefix `LOAD-{SEQ:6}` | 1000 concurrent requests for 5 minutes | - TPS ≥10,000 
- Error rate <0.1% 
- P95 latency <50ms |  
| **Peak traffic** | Register prefix `SPIKE-{SEQ:8}` | Increase from 100 concurrent requests to 5000 in 5 seconds | No request failures, the system automatically recovers to the baseline latency |  
| **Endurance test** | Registered prefix `ENDURANCE-{SEQ:5}` | 500 concurrent sessions for 12 hours | Memory usage is stable (fluctuation <5%) |  
| **Failure recovery** | Manually restart Redis | Continue to request `GET /numbers/RECOVERY` | Service recovered within 30 seconds, no number was missed |  

#### **2.3 Security Testing**  
| Test scenario | Initial data | Operation steps | Assertion data |  
|----------------------|------------------------------------------|----------------------------------|----------------------------------|  
| **Unauthorized access** | No API Token | `POST /prefix-configs/SECURE` | HTTP 401 + error message `Unauthorized` |  
| **SQL injection attack** | `format = "'; DROP TABLE users;--"` | Register prefix | HTTP 400, no changes to database |  
| **Sensitive parameter mask** | Register `initial_seq=9999` | Check log | Log shows `"initial_seq": "****"` |  

---

### **3. Test Data Management**  

#### **3.1 Initial data configuration**  
| Data Type| Source| Example |  
|--------------------|-------------------------------------------|----------------------------------|  
| **Prefix rules** | API `POST /prefix-configs` | `{ "format": "ORD-{SEQ:4}", "seq_length": 4 }` |  
| **Redis preheating data** | Write directly to Redis Hash | `HSET prefix_configs TEST '{"format":"TEST-{SEQ}", ...}'` |  
| **Abnormal simulation** | Mock server (such as `toxiproxy` to simulate network delay) | Set Redis port to 50% packet loss |  

#### **3.2 Asserting Data Types**  
| Types | Examples | Tool Support |  
|--------------------|-------------------------------------------|----------------------------------|  
| **HTTP status code** | `200`, `400`, `409` | Hurl / k6 built-in checks |  
| **JSON structure validation** | `jsonpath "$.number" matches "^ORD-\\d+"`| Hurl `[Asserts]` / k6 `check()` |  
| **Database status** | The value of `sequence:TEST` in Redis is `100` | Custom script verification |  
| **Log Keywords** | Logs contain `"status": "registered"` | ELK / Splunk Query |  

---

### **4. Test execution and exit criteria**  

#### **4.1 Execution Process**  
1. **Unit Testing**:  
   ```bash  
   cargo test --lib # Rust unit tests  
   ```  
2. **E2E functional testing**:  
   ```bash  
   hurl --test --report-html ./reports tests/e2e/*.hurl  
   ```  
3. **Performance test**:  
   ```bash  
   k6 run -o cloud -e K6_CLOUD_TOKEN=tests/load/spike_test.js  
   ```  

#### **4.2 Exit Criteria**  
- **Unit testing**: All tests passed, line coverage ≥ 85%  
- **E2E testing**:  
  - Functional test 100% passed  
  - Performance testing to achieve SLA (error rate, latency, throughput)  
  - Security test with zero high-risk vulnerabilities  

---

### **Appendix: Test Data Example**  
#### **Hurl request/response example**  
```hurl  
# Register prefix  
POST http://localhost:8080/prefix-configs/ORDERS  
{  
  "format": "ORDER-{year}-{SEQ:6}",  
  "seq_length": 6,  
  "initial_seq": 1000  
}  
HTTP/1.1 200  
[Asserts]  
jsonpath "$.status" == "registered"  

# Generate number  
GET http://localhost:8080/numbers/ORDERS  
HTTP/1.1 200  
[Asserts]  
jsonpath "$.number" == "ORDER-2024-001001"  
```  

#### **k6 threshold configuration example**  
```javascript  
export const options = {  
  thresholds:  
    http_req_duration: ['p(95)<100', 'p(99)<200'],  
    http_req_failed: ['rate<0.01']  
  }  
};  
```