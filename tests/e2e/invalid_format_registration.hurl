# Invalid format registration
PUT http://localhost:8080/api/prefix-configs/INVALID
Content-Type: application/json

{
  "format": "INVALID",
  "seqLength": 4,
  "initialSeq": 1
}

HTTP/1.1 400
