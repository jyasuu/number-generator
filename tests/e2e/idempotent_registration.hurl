# Register prefix
PUT http://localhost:8080/api/prefix-configs/DUPLICATE
Content-Type: application/json

{
  "format": "DUP-{year}-{SEQ:4}",
  "seqLength": 4,
  "initialSeq": 1
}

HTTP/1.1 200

# Repeat registration of the same prefix
PUT http://localhost:8080/api/prefix-configs/DUPLICATE
Content-Type: application/json

{
  "format": "DUP-{year}-{SEQ:4}",
  "seqLength": 4,
  "initialSeq": 1
}

HTTP/1.1 409
