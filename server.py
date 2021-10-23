import socket
import time
import pyaudio
import wave
import struct
import numpy as np

CHUNK = 512
FORMAT = pyaudio.paInt16
CHANNELS = 2
RATE = 44100

p = pyaudio.PyAudio()

stream = p.open(format=FORMAT,
                channels=CHANNELS,
                rate=RATE,
                input=True,
                frames_per_buffer=CHUNK)

print("* recording")

frames = []

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
# sock.bind(("0.0.0.0", 4050))
# sock.listen(10)
i = 0
while 1:
    try:
        data = stream.read(CHUNK)
        sock.sendto(data, ("192.168.0.13", 4051))
    except:
        pass