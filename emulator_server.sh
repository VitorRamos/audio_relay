#!/bin/bash

token=$(cat $HOME/.emulator_console_auth_token)

telnet localhost 5554 <<EOF
help
auth $token
redir add udp:4051:4051
redir add udp:4052:4052
redir add udp:4053:4053
EOF
