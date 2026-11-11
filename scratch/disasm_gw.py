with open('driveD/GWBASIC.EXE', 'rb') as f:
    data = f.read()

idx = 0x9d97
print(list(data[idx-30:idx+10]))
