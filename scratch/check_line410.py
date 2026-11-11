with open('driveD/cube2.bas', 'rb') as f:
    data = f.read()

idx = data.find(b"PRESS")
print("Found PRESS at", idx)
print("Bytes around PRESS:", list(data[idx-15:idx+30]))
