# Register prefix
PUT http://localhost:8080/api/prefix-configs/ORDER
Content-Type: application/json

{
  "format": "ORDER-{year}-{SEQ:6}",
  "seqLength": 6,
  "initialSeq": 1000
}

HTTP/1.1 200

# Generate number
GET http://localhost:8080/api/numbers/ORDER

HTTP/1.1 200
Content-Type: application/json

[Asserts]
jsonpath "$.number" matches "ORDER-\\d{4}-\\d{6}"

# Set network partition
POST http://localhost:8080/api/prefix-configs/ORDER/network-partition

HTTP/1.1 200

# Generate number with network partition
GET http://localhost:8080/api/numbers/ORDER

HTTP/1.1 200
Content-Type: application/json

[Asserts]
jsonpath "$.number" matches "ORDER-\\d{4}-\\d{6}-NP"
