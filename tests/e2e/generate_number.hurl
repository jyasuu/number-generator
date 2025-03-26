# Register prefix
POST http://localhost:8080/api/prefix-configs/ORDERS
Content-Type: application/json

{
  "format": "ORDER-{year}-{SEQ:6}",
  "seqLength": 6,
  "initialSeq": 1000
}

HTTP/1.1 200

# Generate number
GET http://localhost:8080/api/numbers/ORDERS

HTTP/1.1 200
Content-Type: application/json

[Asserts]
jsonpath "$.number" matches "ORDER-\\d{4}-\\d{6}"
