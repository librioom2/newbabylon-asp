#!/bin/bash
# Generate self-signed TLS certificates for quiche (QUIC) demo
# Usage: cd certs && bash generate.sh

set -euo pipefail

CERT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "🔐 Generating self-signed TLS certificate for quiche..."

openssl req -x509 \
  -newkey ec \
  -pkeyopt ec_paramgen_curve:prime256v1 \
  -keyout "${CERT_DIR}/key.pem" \
  -out "${CERT_DIR}/cert.pem" \
  -days 365 \
  -nodes \
  -subj "/CN=localhost"

echo "✅ Certificates generated:"
echo "   ${CERT_DIR}/cert.pem"
echo "   ${CERT_DIR}/key.pem"
echo ""
echo "Use with quiche Config:"
echo '   config.load_cert_chain_from_pem_file("certs/cert.pem")?;'
echo '   config.load_priv_key_from_pem_file("certs/key.pem")?;'
