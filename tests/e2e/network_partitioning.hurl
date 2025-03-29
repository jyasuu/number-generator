# Register prefix
POST http://localhost:8080/api/prefix-configs/NP
Content-Type: application/json

{
  "format": "NP-{year}-{SEQ:4}",
  "seqLength": 4,
  "initialSeq": 1
}

HTTP/1.1 200

# Set network partition
POST http://localhost:8080/api/prefix-configs/NP/network-partition

HTTP/1.1 200

# Generate number with network partition
GET http://localhost:8080/api/numbers/NP

HTTP/1.1 200
Content-Type: application/json

[Asserts]
jsonpath "$.number" matches "NP-\\d{4}-\\d{4}-NP"
