#!/bin/sh
cd ~
openssl genrsa -out server_rsa.pem 1024
openssl rsa -in server_rsa.pem -inform PEM -outform DER -out server_rsa
openssl rsa -in server_rsa.pem -inform PEM -outform DER -pubout -out server_rsa.pub
racemus
