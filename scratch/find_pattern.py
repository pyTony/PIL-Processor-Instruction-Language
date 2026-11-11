with open('driveD/GWBASIC.EXE', 'rb') as f:
    data = f.read()

pattern = bytes([0xCD, 0x10, 0x5F, 0x5E, 0x5D, 0xC3])
idx = 0
while True:
    idx = data.find(pattern, idx)
    if idx == -1: break
    print(f"Found pattern at {hex(idx)}")
    idx += 1
