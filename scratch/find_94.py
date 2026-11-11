with open('driveD/cube2.bas', 'rb') as f:
    data = f.read()
indices = [i for i, b in enumerate(data) if b == 0x94]
print('0x94 occurrences at indices:', indices)
for idx in indices:
    print('context:', data[max(0, idx-10):min(len(data), idx+10)])
