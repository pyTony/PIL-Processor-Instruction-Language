with open('driveD/GWBASIC.EXE', 'rb') as f:
    data = f.read()

# MZ header
header_paras = int.from_bytes(data[8:10], 'little')
header_size = header_paras * 16
code = data[header_size:]

target_offset = 0x6CC7
calls = []
for i in range(len(code) - 3):
    if code[i] == 0xE8:
        disp = int.from_bytes(code[i+1:i+3], 'little', signed=True)
        dest = i + 3 + disp
        if dest == target_offset:
            calls.append(i)

print(f"Header size: {hex(header_size)}")
print(f"Calls to 0x6CC7: {[hex(c) for c in calls]}")
for c in calls:
    print(f"Context before call at {hex(c)}:", list(code[max(0, c-20):c+3]))
