# number-generator

---

## **Dynamic Unique Number Generation Service Design Document**

---

### **1. Functional Overview**
#### **Objective**  
Design an extensible distributed unique number generation service with the following core capabilities:  
1. **Dynamic Prefix Rules**: Allow flexible definition of number formats (e.g., embedding dates, business tags) without code modification.  
2. **Efficient Concurrency Control**: Ensure globally unique and sequential numbers in high-concurrency scenarios (configurable for strict sequence).  
3. **No Single Point of Failure**: Support distributed deployment to avoid dependency on a single service or data store.  

#### **Non-Functional Requirements**  
- **Throughput**: Single node supports ≥ 10,000 TPS.  
- **Latency**: Average response time ≤ 5ms.  
- **Fault Tolerance**: Service can automatically degrade or recover when middleware (e.g., Redis) is temporarily unavailable.  

---

### **2. Architecture Design**  
#### **System Components**  
| Component              | Description                               | Technology Implementation         |  
|------------------------|-------------------------------------------|-----------------------------------|  
| **Prefix Rule Manager** | Stores and manages dynamic prefix formats and sequence configurations | Redis Hash / Relational Database |  
| **Sequence Generator**  | Generates unique incrementing numbers with atomic operations          | Redis INCR / Distributed Lock     |  
| **Number Assembler**    | Formats prefix and sequence values into final numbers according to rules | Application layer string processing logic |  
| **Monitoring & Alerting** | Collects performance metrics and monitors number generation health    | Prometheus + Grafana             |  

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
- **Operations Interface**  
  - **Register Rule**: `PUT /prefix-configs/{prefixKey}`  
  - **Query Rule**: `GET /prefix-configs/{prefixKey}`  

#### **Module 2: Sequence Generator**  
- **Concurrency Control Strategies**  
  | Scenario                  | Strategy                              | Use Case                |  
  |---------------------------|---------------------------------------|-------------------------|  
  | Low Latency + Allow Minor Number Gaps | Redis Atomic Operation (`INCR`)           | High Throughput (Default)  |  
  | Strict Sequence           | Database Row Lock + Optimistic Lock Retry | Financial Transactions etc. |  
  | Distributed High Availability | Modified Snowflake Algorithm           | Cross Data Center Deployment |  

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

### **5. Performance & Scalability**  
#### **Load Test Results**  
| Scenario                | Concurrency | Average TPS | 95% Latency (ms) |  
|-------------------------|-------------|-------------|------------------|  
| Redis Single Node       | 1000        | 12,000      | 8                |  
| Database + Optimistic Lock | 1000        | 850         | 120              |  

#### **Horizontal Scaling Solutions**  
1. **Redis Cluster**: Distributed storage of prefix rules and sequence values, shard key = `prefixKey`.  
2. **Stateless Services**: Deploy multiple number generation service instances, distribute requests via load balancer.  

---

### **6. Fault Tolerance Design**  
#### **Failure Scenario Handling**  
| Failure Type              | Degradation Strategy                  | Recovery Mechanism        |  
|---------------------------|---------------------------------------|---------------------------|  
| Redis Unavailable         | Switch to local number segment cache (pre-allocated 1000 numbers) | Periodic retry connection, incremental sync |  
| Service Node Down         | Traffic automatically switches to healthy nodes | K8s auto-restart containers |  
| Network Partition         | Use local clock to generate temporary numbers (with marker) | Manual conflict resolution |  

---

### **7. Monitoring Metrics**  
| Metric Name                  | Monitoring Target                      | Alert Threshold          |  
|------------------------------|---------------------------------------|--------------------------|  
| `numbergen_request_rate`     | Real-time generation request rate      | > 10,000/min            |  
| `numbergen_error_ratio`      | Generation failure rate (including unregistered prefix errors) | > 1% (for 5 minutes) |  
| `redis_connection_latency`   | Redis operation latency                | > 50ms                  |  

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

### **Conclusion**  
This design document focuses on technical implementation without coupling to specific business logic, serving as a reference architecture for a generic unique number generation service. The core values are:  
1. **Elastic Scalability**: Dynamic prefix rules and distributed architecture support rapid business changes.  
2. **Stable & Efficient**: Multiple fault tolerance mechanisms ensure service SLA.  
3. **Non-intrusive Integration**: Provides service through standard API, business side doesn't need to know internal implementation.
