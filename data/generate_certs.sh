#!/usr/bin/env bash
cd "$(dirname "${BASH_SOURCE[0]}")"

set -ex
rm -f *.csr *.key *.pem *.srl

# Server CA
openssl genrsa -out server_ca.key 3072
openssl req -x509 -new -sha256 -days 1000 \
    -subj "/CN=Paas Server Certificate Authority" \
    -key server_ca.key -out server_ca.pem

# Server, signed by server CA
openssl genrsa -out server.key 3072
openssl req -new -sha256 \
    -subj "/CN=server" \
    -key server.key -out server.csr

openssl x509 -req -sha256 -days 1000 \
    -CA server_ca.pem -CAkey server_ca.key -CAcreateserial \
    -extfile server.ext \
    -in server.csr -out server.pem \

# Client CA
openssl genrsa -out client_ca.key 3072
openssl req -x509 -new -sha256 -days 1000 \
    -subj "/CN=Paas Client Certificate Authority" \
    -key client_ca.key -out client_ca.pem

# Clients, signed by client CA
openssl genrsa -out client1.key 3072
openssl genrsa -out client2.key 3072

openssl req -new -sha256 -days 1000 -subj "/CN=client1" -key client1.key -out client1.csr
openssl req -new -sha256 -days 1000 -subj "/CN=client2" -key client2.key -out client2.csr

openssl x509 -req -sha256 -days 1000 \
    -CA client_ca.pem -CAkey client_ca.key -CAcreateserial \
    -extfile client.ext \
    -in client1.csr -out client1.pem

openssl x509 -req -sha256 -days 1000 \
    -CA client_ca.pem -CAkey client_ca.key \
    -extfile client.ext \
    -in client2.csr -out client2.pem
