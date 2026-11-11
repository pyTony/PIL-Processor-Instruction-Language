"""Small byte/fixup builder for original DOS fixtures, with matching NASM text."""


class Image:
    def __init__(self, lines=()):
        self.code, self.labels, self.fixups = bytearray(), {}, []
        self.lines = list(lines)

    def label(self, name):
        self.labels[name] = len(self.code)
        self.lines.append(name + ':')

    def emit(self, hexcode, text):
        self.code.extend(bytes.fromhex(hexcode))
        self.lines.append('    ' + text)

    def address(self, before, label, after, text):
        self.emit(before, text)
        self.fixups.append((len(self.code), label, 2, False))
        self.code.extend(b'\0\0' + bytes.fromhex(after))

    def branch(self, opcode, label, text, width=2):
        self.emit(opcode, text)
        self.fixups.append((len(self.code), label, width, True))
        self.code.extend(bytes(width))

    def data(self, label, data):
        self.label(label)
        self.code.extend(data)
        self.lines.append('    db ' + ','.join(str(b) for b in data))

    def finish(self):
        for at, label, width, relative in self.fixups:
            value = self.labels[label] - at - width if relative else 0x100 + self.labels[label]
            self.code[at:at + width] = value.to_bytes(width, 'little', signed=relative)
        return bytes(self.code), '\n'.join(self.lines) + '\n'
