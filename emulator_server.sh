#!/bin/bash

token=$(cat /home/vitor/.emulator_console_auth_token)

telnet localhost 5554 <<EOF
help
auth $token
redir add udp:4051:4051
EOF
