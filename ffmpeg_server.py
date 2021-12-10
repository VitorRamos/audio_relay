import subprocess as sp
import os
import select

"ffmpeg -f pulse -i default -f mp3 -ar 44100 -ac 2 -b:a 192k"
"ffmpeg -f pulse -i default -f wav -sample_fmt s16 -ar 44100 -ac 2 -"

RATE = "44100"
FORMAT = "s16"

p = sp.Popen(["ffmpeg", 
                "-f", "pulse",
                "-i", "default",
                "-f", "wav",
                "-vn",
                "-sample_fmt", FORMAT,
                "-ar", RATE,
                "-ac", "2",
                "pipe:2"])

# p.stdin.close()
# p.wait()
while p.poll() is not None:
    data = p.stdout.readline()
    print(data)