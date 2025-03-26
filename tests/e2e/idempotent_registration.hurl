# Register prefix
POST http://localhost:8080/api/prefix-configs/DUPLICATE
Content-Type: application/json

{
  "format": "DUP-{SEQ:4}",
  "seqLength": 4,
  "initialSeq": 1
}

HTTP/1.1 200

# Repeat registration of the same prefix
POST http://localhost:8080/api/prefix-configs/DUPLICATE
Content-Type: application/json

{
  "format": "DUP-{SEQ:4}",
  "seqLength": 4,
  "initialSeq": 1
}

HTTP/1.1 409
