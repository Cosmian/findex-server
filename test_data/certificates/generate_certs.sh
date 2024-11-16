#!/bin/bash

# Generate CA private key
openssl genpkey -algorithm RSA -out ca.key

# Generate self-signed CA certificate
openssl req -new -x509 -days 3650 -key ca.key -subj "/C=FR/ST=IdF/L=Paris/O=AcmeTest/CN=Acme Test Root CA" -out ca.crt


## Server Cert

# Generate private key for findex.server.acme.com
openssl genpkey -algorithm RSA -out findex.server.acme.com.key

# Generate certificate signing request for findex.server.acme.com
openssl req -new -key findex.server.acme.com.key -subj "/C=FR/ST=IdF/L=Paris/O=AcmeTest/CN=findex.server.acme.com" -out findex.server.acme.com.csr

# Generate certificate for findex.server.acme.com signed by our own CA
openssl x509 -req -days 3650 -in findex.server.acme.com.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out findex.server.acme.com.crt

# Generate a PKCS12 file
openssl pkcs12 -export -out findex.server.acme.com.p12 -inkey findex.server.acme.com.key -in findex.server.acme.com.crt -certfile ca.crt -password pass:password


## "owner" client cert

# Generate private key for owner.client.acme.com
openssl genpkey -algorithm RSA -out owner.client.acme.com.key

# Generate certificate signing request for owner.client.acme.com
openssl req -new -key owner.client.acme.com.key -subj "/C=FR/ST=IdF/L=Paris/O=AcmeTest/CN=owner.client@acme.com" -out owner.client.acme.com.csr

# Generate certificate for owner.client.acme.com signed by our own CA
openssl x509 -req -days 3650 -in owner.client.acme.com.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out owner.client.acme.com.crt

# Generate a PKCS12 file
openssl pkcs12 -export -out owner.client.acme.com.p12 -inkey owner.client.acme.com.key -in owner.client.acme.com.crt -certfile ca.crt -password pass:password
openssl pkcs12 -legacy -export -out owner.client.acme.com.old.format.p12 -inkey owner.client.acme.com.key -in owner.client.acme.com.crt -certfile ca.crt -password pass:password

## "user" client cert

# Generate private key for user.client.acme.com
openssl genpkey -algorithm RSA -out user.client.acme.com.key

# Generate certificate signing request for user.client.acme.com
openssl req -new -key user.client.acme.com.key -subj "/C=FR/ST=IdF/L=Paris/O=AcmeTest/CN=user.client@acme.com" -out user.client.acme.com.csr

# Generate certificate for user.client.acme.com signed by our own CA
openssl x509 -req -days 3650 -in user.client.acme.com.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out user.client.acme.com.crt

# Generate a PKCS12 file
openssl pkcs12 -export -out user.client.acme.com.p12 -inkey user.client.acme.com.key -in user.client.acme.com.crt -certfile ca.crt -password pass:password
openssl pkcs12 -legacy -export -out user.client.acme.com.old.format.p12 -inkey user.client.acme.com.key -in user.client.acme.com.crt -certfile ca.crt -password pass:password
