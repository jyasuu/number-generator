# Register prefix A
PUT http://localhost:8080/api/prefix-configs/A
Content-Type: application/json

{
  "format": "A-{year}-{SEQ:3}",
  "seqLength": 3,
  "initialSeq": 1
}

HTTP/1.1 200

# Register prefix B
PUT http://localhost:8080/api/prefix-configs/B
Content-Type: application/json

{
  "format": "B-{year}-{SEQ:3}",
  "seqLength": 3,
  "initialSeq": 1
}

HTTP/1.1 200

# Generate number for prefix A
GET http://localhost:8080/api/numbers/A

HTTP/1.1 200
Content-Type: application/json

[Asserts]
jsonpath "$.number" == "A-2025-001"

# Generate number for prefix B
GET http://localhost:8080/api/numbers/B

HTTP/1.1 200
Content-Type: application/json

[Asserts]
jsonpath "$.number" == "B-2025-001"

# Generate number for prefix A again
GET http://localhost:8080/api/numbers/A

HTTP/1.1 200
Content-Type: application/json

[Asserts]
jsonpath "$.number" == "A-2025-002"

# Generate number for prefix B again
GET http://localhost:8080/api/numbers/B

HTTP/1.1 200
Content-Type: application/json

[Asserts]
jsonpath "$.number" == "B-2025-002"
