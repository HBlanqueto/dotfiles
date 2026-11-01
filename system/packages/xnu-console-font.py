import re
import struct
import sys

src = open(sys.argv[1]).read()

body = src[src.index("iso_font[256 * 16] = {"):]
body = body[body.index("{") + 1:body.index("};")]
raw = bytes(int(v, 16) for v in re.findall(r"0x[0-9a-fA-F]{2}\b", body))
if len(raw) != 4096:
    sys.exit(f"expected 4096 font bytes, got {len(raw)}")

def reverse_bits(b):
    return int(f"{b:08b}"[::-1], 2)

data = bytes(reverse_bits(b) for b in raw)

with open("xnu-console.psf", "wb") as f:
    f.write(struct.pack("<IIIIIIII", 0x864AB572, 0, 32, 1, 256, 16, 16, 8))
    f.write(data)
    for i in range(256):
        f.write(chr(i).encode("utf-8") + b"\xff")
