"""Independent expected-value checks of extension/address PIL descriptions."""
from verify_descriptions import tokens,interpret

count=0
for source in (8,16):
    for dest in (16,32):
        if source>=dest:continue
        for signed in (False,True):
            code=tokens(('movsx' if signed else 'movzx')+str(source),dest)
            for n in range(1<<source):
                # Arithmetic range conversion, separate from PIL sext bit operations.
                mathematical=n-(1<<source) if signed and n>=(1<<(source-1)) else n
                mask=(1<<dest)-1
                expected=(0xabcd5678 & ~mask)|(mathematical & mask)
                assert interpret(code,0xabcd5678,n,dest,0xad7,0x8d5)==(expected,0xad7,0x8d5)
                count+=1
for dest in (16,32):
    code=tokens('cwd',dest);mask=(1<<dest)-1
    for n in (0,1,(1<<(dest-1))-1,1<<(dest-1),mask):
        expected=(0xabcd5678 & ~mask)|(mask if n>=(1<<(dest-1)) else 0)
        assert interpret(code,0xabcd5678,n,dest,0xad7,0x8d5)==(expected,0xad7,0x8d5)
        count+=1
    code=tokens('lea',dest)
    for offset in (0,1,0xffff,0x10000,0x80000000,0xffffffff):
        expected=(0xabcd5678 & ~mask)|(offset & mask)
        assert interpret(code,0xabcd5678,0,dest,0xad7,0x8d5,ram=bytearray(),address=offset)==(expected,0xad7,0x8d5)
        count+=1
print(f'{count:,} independent extension/address PIL cases passed; Rust decode is a separate check.')
