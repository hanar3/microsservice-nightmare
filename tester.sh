#!/bin/bash

# Define the target IP and port
IP=127.0.0.1
PORT=8080

# Define the buffer to send
BUFFER="\x01\x0C\x04\x54\x45\x53\x54\x01\x00\x04\x65\x63\x68\x6F"

# Send the buffer to the server using netcat
echo -ne $BUFFER | nc $IP $PORT
