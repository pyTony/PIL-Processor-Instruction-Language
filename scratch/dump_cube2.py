with open('driveD/cube2.bas', 'rb') as f:
    data = f.read()

tokens = {
    0x81: "END", 0x82: "FOR", 0x83: "NEXT", 0x84: "DATA", 0x85: "INPUT",
    0x86: "DIM", 0x87: "READ", 0x88: "LET", 0x89: "GOTO", 0x8a: "RUN",
    0x8b: "IF", 0x8c: "RESTORE", 0x8d: "GOSUB", 0x8e: "RETURN", 0x8f: "REM",
    0x91: "PRINT", 0x95: "COLOR", 0x96: "CLS", 0x9b: "LOCATE",
    0xaf: "THEN", 0xb1: "STEP", 0xb2: "+", 0xb3: "-", 0xb4: "*", 0xb5: "/",
    0xb8: ">", 0xb9: "=", 0xba: "<", 0xc0: "SCREEN", 0xc2: "LINE",
    0xc6: "SIN", 0xc7: "COS",
}

p = 1
while p < len(data) - 1:
    next_ptr = data[p] | (data[p+1] << 8)
    if next_ptr == 0: break
    line_num = data[p+2] | (data[p+3] << 8)
    p += 4
    chars = []
    while p < len(data) and data[p] != 0:
        b = data[p]
        p += 1
        if b in tokens:
            chars.append(f" {tokens[b]} ")
        elif b >= 0x80:
            chars.append(f"<{b:02x}>")
        else:
            chars.append(chr(b))
    p += 1
    print(f"{line_num}: {''.join(chars)}")
