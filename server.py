from typing import Tuple
import socket
import pyaudio
import time

def wait_for_server_adress(port: str) -> str:
    print("Waiting for server adress")
    bcast_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    bcast_sock.bind(("", port))
    while 1:
        data = bcast_sock.recvfrom(12)
        if data:
            print("Server adress recvied", data[1][0])
            server_adress = data[1][0]
            break
    return server_adress

def create_audio_stream(format: str, channels: str, rate: str, chunk: str) -> pyaudio.PyAudio:
    print("Creating audio stream")

    p = pyaudio.PyAudio()
    stream = p.open(format=format,
                    channels=channels,
                    rate=rate,
                    input=True,
                    frames_per_buffer=chunk)

    return stream

def send_audio(stream: pyaudio.PyAudio, chunk: int, rate: int, adress: Tuple[str, str]):
    print("Recording...")
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    ti = time.time()
    while 1:
        try:
            data = stream.read(CHUNK)
            if data == b"\x00"*len(data):
                continue
            sock.sendto(data, adress)
            ep_time = time.time()-ti
            if ep_time < chunk/rate:
                time.sleep(chunk/rate*0.9-ep_time)
            print(time.time()-ti)
            ti = time.time()
        except KeyboardInterrupt:
            break
        except:
            pass

CHUNK = 512
RATE = 44100
PORT_BCAST = 4052
PORT_SERVER = 4051

server_adress = wait_for_server_adress(port=PORT_BCAST)
stream = create_audio_stream(format=pyaudio.paInt16, channels=2, rate=RATE, chunk=CHUNK)
send_audio(stream=stream, chunk=CHUNK, rate=RATE, adress=(server_adress, PORT_SERVER))