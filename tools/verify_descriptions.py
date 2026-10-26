"""Independent PIL-description arithmetic check; does not compile/test Rust."""
from pathlib import Path

BASE = Path(__file__).resolve().parents[1] / "isa" / "i386"
OPS = ("add", "sub", "cmp", "adc", "sbb", "and", "or", "xor", "test")
CONDITIONS = ("o","no","b","ae","e","ne","be","a","s","ns","p","np","l","ge","le","g")
FLAGS = {"CF": 0, "PF": 2, "AF": 4, "ZF": 6, "SF": 7, "OF": 11}
ARITHMETIC_MASK = 0x8D5


def tokens(op, width):
    if op.startswith("j"):
        condition=op[1:]
        assert condition in CONDITIONS
        text=(BASE / "conditions.pil").read_text()+"\n"+(BASE / "jcc.pil").read_text().replace("$CONDITION","cond_"+condition)
    elif op.startswith("set"):
        condition = op[3:]
        assert condition in CONDITIONS
        text = (BASE / "conditions.pil").read_text()+"\n"+(BASE / "setcc.pil").read_text().replace("$CONDITION", "cond_"+condition)
    else:
        text = (BASE / (op + ".pil")).read_text()
    for name, value in (("$DOUBLE", width * 2), ("$EXT", width + 1), ("$SIGN", width - 1), ("$W", width)):
        text = text.replace(name, str(value))
    return [line.split("#", 1)[0].split() for line in text.splitlines()
            if line.split("#", 1)[0].strip()]


class FlowFault(Exception):
    def __init__(self, eip):
        self.eip=eip
        self.error_code=0
        super().__init__("#GP(0)")


class MemoryFault(Exception):
    def __init__(self,eip,address,bytes,write):
        self.eip,self.address,self.bytes,self.write=eip,address,bytes,write
        super().__init__("flat RAM access fault")


def interpret(code, a, b, width, initial, initial_undefined=0, flow=None, current_eip=0, ram=None, address=0, readonly=()):
    pending_store=None
    uses_memory=any(t[0] in ("read_address","load","store") for t in code)
    if uses_memory and ram is None:
        raise ValueError("RAM context required")
    def check_memory(addr,size,write):
        if addr<0 or addr+size>len(ram) or addr+size>2**32 or (write and any(addr<end and addr+size>start for start,end in readonly)):
            raise MemoryFault(current_eip,addr,size,write)
    uses_flow=any(t[0] in ("read_ip","read_disp","branch_if") for t in code)
    if uses_flow and flow is None:
        raise ValueError("flow context required")
    if flow is not None and any(t[0]=="read_disp" for t in code):
        bits=flow["bits"];d=flow["displacement"]
        if bits not in (8,16,32) or not -(1 << (bits-1)) <= d < (1 << (bits-1)) or (bits!=8 and bits!=width):
            raise ValueError("invalid displacement")
    pending_eip=flow["next"] if flow is not None else current_eip
    values = {}
    destination, flags = a, initial
    undefined = initial_undefined
    classified = 0
    for t in code:
        op = t[0]
        if op == "retire":
            assert classified == ARITHMETIC_MASK
            if pending_store is not None:
                addr,data=pending_store
                check_memory(addr,len(data),True)
                ram[addr:addr+len(data)]=data
            result=(destination, flags, undefined)
            return result if flow is None else result+(pending_eip,)
        if op == "require_width":
            assert width in (16,32) if t[1]=="16_or_32" else int(t[1])==width
        elif op == "read_address":
            values[t[1]]=(address,32)
        elif op == "load":
            addr,aw=values[t[2]];w=int(t[3]);assert aw==32 and w in (8,16,32)
            check_memory(addr,w//8,False)
            values[t[1]]=(int.from_bytes(ram[addr:addr+w//8],"little"),w)
        elif op == "store":
            addr,aw=values[t[1]];n,w=values[t[2]];assert aw==32 and w in (8,16,32)
            check_memory(addr,w//8,True)
            pending_store=(addr,n.to_bytes(w//8,"little"))
        elif op == "read_ip":
            w=int(t[3]);n=flow["next"] if t[2]=="next" else current_eip
            values[t[1]]=(n & ((1<<w)-1),w)
        elif op == "read_disp":
            w=int(t[2]);assert w==width and w in (16,32)
            values[t[1]]=(flow["displacement"] & ((1<<w)-1),w)
        elif op == "branch_if":
            n,w=values[t[1]];c,cw=values[t[2]]
            assert w==32 and cw==1
            if c:
                if n>flow["limit"]:
                    raise FlowFault(current_eip)
                pending_eip=n
        elif op == "preserve_flag":
            m=1 << FLAGS[t[1]]
            assert not classified & m
            classified |= m
        elif op == "preserve_flags":
            assert t[1] in ("all", "remaining")
            m=ARITHMETIC_MASK if t[1]=="all" else ARITHMETIC_MASK & ~classified
            assert not classified & m
            classified |= m
        elif op == "undef_flag":
            m=1 << FLAGS[t[1]]
            assert not classified & m
            classified |= m
            flags &= ~m
            undefined |= m
        elif op == "flag_if":
            n,nw=values[t[2]];defined,dw=values[t[3]];active,cw=values[t[4]]
            assert (nw,dw,cw)==(1,1,1)
            m=1<<FLAGS[t[1]];assert not classified&m;classified|=m
            if active:
                flags &= ~m
                if defined:flags|=n<<FLAGS[t[1]];undefined&=~m
                else:undefined|=m
        elif op == "ult":
            x,w=values[t[2]];y,yw=values[t[3]];assert w==yw
            values[t[1]]=(int(x<y),1)
        elif op in ("shl","lshr","ashr"):
            x,w=values[t[2]];n,_=values[t[3]]
            if op=="ashr":
                if x&(1<<(w-1)):x-=1<<w
                result=x>>min(n,w-1)
            elif n>=w:result=0
            else:result=x<<n if op=="shl" else x>>n
            values[t[1]]=(result&((1<<w)-1),w)
        elif op == "select":
            c,cw=values[t[2]]
            assert cw==1 and values[t[3]][1]==values[t[4]][1]
            values[t[1]]=values[t[3]] if c else values[t[4]]
        elif op == "write_if":
            c,cw=values[t[3]]
            n,w=values[t[2]]
            assert cw==1 and w==width
            if c:
                destination=(a & ~((1 << width)-1)) | n
        elif op == "read":
            w = int(t[3])
            values[t[1]] = ((a if t[2] == "dst" else b) & ((1 << w) - 1), w)
        elif op == "const":
            w,n=int(t[2]),int(t[3],0)
            assert 0 <= n < (1 << w)
            values[t[1]]=(n,w)
        elif op == "read_flag":
            values[t[1]] = ((initial >> FLAGS[t[2]]) & 1, 1)
        elif op == "flag":
            n, w = values[t[2]]
            assert w == 1
            bit = FLAGS[t[1]]
            m=1 << bit
            assert not classified & m
            classified |= m
            undefined &= ~m
            flags = (flags & ~(1 << bit)) | (n << bit)
        elif op == "write":
            n, w = values[t[2]]
            assert w == width
            destination = (a & ~((1 << width) - 1)) | n
        elif op in ("zext", "sext", "trunc"):
            n, old_width = values[t[2]]
            w = int(t[3])
            assert (w >= old_width) if op != "trunc" else (w <= old_width)
            if op == "sext" and n & (1 << (old_width-1)):
                n -= 1 << old_width
            values[t[1]] = (n & ((1 << w) - 1), w)
        elif op in ("add", "sub", "mul", "xor", "and", "or"):
            x, w = values[t[2]]
            y, other_width = values[t[3]]
            assert w == other_width
            if op == "add":
                n = x + y
            elif op == "sub":
                n = x - y
            elif op == "mul":
                n = x * y
            elif op == "xor":
                n = x ^ y
            elif op == "or":
                n = x | y
            else:
                n = x & y
            values[t[1]] = (n & ((1 << w) - 1), w)
        elif op == "not":
            n, w = values[t[2]]
            values[t[1]] = (~n & ((1 << w) - 1), w)
        elif op == "bit":
            n, w = values[t[2]]
            index = int(t[3])
            assert index < w
            values[t[1]] = ((n >> index) & 1, 1)
        elif op == "zero":
            values[t[1]] = (int(values[t[2]][0] == 0), 1)
        elif op == "parity":
            values[t[1]] = (int((values[t[2]][0] & 255).bit_count() % 2 == 0), 1)
        else:
            raise AssertionError(t)
    raise AssertionError("missing retirement")


def oracle(op, a, b, width, initial):
    mask = (1 << width) - 1
    x, y = a & mask, b & mask
    carry = (initial & 1) if op in ("adc", "sbb") else 0
    subtract = op in ("sub", "cmp", "sbb")
    logical = op in ("and", "or", "xor", "test")
    total = ((x | y) if op=="or" else (x ^ y) if op=="xor" else (x & y)) if logical else (x - y - carry if subtract else x + y + carry)
    result = total & mask
    def signed(n):
        return n - (1 << width) if n & (1 << (width - 1)) else n
    signed_total = signed(x) - signed(y) - carry if subtract else signed(x) + signed(y) + carry
    cf = False if logical else (x < y + carry if subtract else total > mask)
    af = (x & 15) < (y & 15) + carry if subtract else (x & 15) + (y & 15) + carry > 15
    of = not logical and not -(1 << (width - 1)) <= signed_total < (1 << (width - 1))
    if logical:
        af=False
    flags = (int(cf) | (int((result & 255).bit_count() % 2 == 0) << 2)
             | (int(af) << 4) | (int(result == 0) << 6)
             | (((result >> (width - 1)) & 1) << 7) | (int(of) << 11))
    return (a if op in ("cmp", "test") else (a & ~mask) | result,
            (initial & ~ARITHMETIC_MASK) | flags,
            16 if logical else 0)


if __name__ == "__main__":
    count = 0
    for op in OPS:
        for width in (8, 16, 32):
            code = tokens(op, width)
            mask, sign = (1 << width) - 1, 1 << (width - 1)
            values = range(256) if width == 8 else (0, 1, 15, 16, sign - 1, sign, mask)
            initial_flags = (0x202, 0x203) if width == 8 else (0, 1, 0x202, 0x203, 0xFFFFFFFF)
            for initial in initial_flags:
                for a in values:
                    for b in values:
                        got = interpret(code, a, b, width, initial)
                        want = oracle(op, a, b, width, initial)
                        assert got == want, (op, width, a, b, initial, got, want)
                        count += 1
            # Preserve parent register bits for byte/word destinations.
            if width < 32:
                a = 0xABCDFFFF
                assert interpret(code, a, 1, width, 0x203) == oracle(op, a, 1, width, 0x203)
        print(op.upper(), "description verified")
    imul_count=0
    for width in (16,32):
        code=tokens("imul",width);mask=(1<<width)-1;sign=1<<(width-1)
        values=list(range(-128,128)) if width==16 else [-sign,-sign+1,-65536,-32768,-129,-1,0,1,127,32767,65536,sign-1]
        for x in values:
            for y in values:
                a,b=x&mask,y&mask;product=x*y;overflow=not -sign<=product<sign
                got=interpret(code,a,b,width,0xffffffff,0x8d5)
                want=(product&mask,(0xffffffff&~0x8d5)|(0x801 if overflow else 0),0xd4)
                assert got==want,(width,x,y,got,want)
                imul_count+=1
    print(f"{imul_count:,} independent signed IMUL cases passed.")
    shift_count=0
    for op in ("shl","shr","sar"):
        for width in (8,16,32):
            code=tokens(op,width);mask=(1<<width)-1;sign=1<<(width-1)
            values=range(256) if width==8 else (0,1,3,sign-1,sign,mask,0xa55aa55a)
            for a in values:
                for raw in list(range(32))+[32,33,63,64,255]:
                    for initial,u in ((0x202,0),(0xad7,0x8d5)):
                        n=raw&31;x=a&mask
                        if not n:want=(a,initial,u)
                        else:
                            result=((x<<n) if op=="shl" else (x>>n) if op=="shr" else ((x-(1<<width) if x&sign else x)>>n))&mask
                            cf_undefined=op!="sar" and n>=width
                            cf=0 if cf_undefined else (x>>(width-n))&1 if op=="shl" else int(bool(x&sign)) if n>=width else (x>>(n-1))&1
                            of=0 if n!=1 or op=="sar" else int(bool((x^result)&sign)) if op=="shl" else int(bool(x&sign))
                            flags=(initial&~0x8d5)|cf|(int((result&255).bit_count()%2==0)<<2)|(int(result==0)<<6)|(int(bool(result&sign))<<7)|(of<<11)
                            want=((a&~mask)|result,flags,16|(0x800 if n>1 else 0)|int(cf_undefined))
                        got=interpret(code,a,raw,width,initial,u)
                        assert got==want,(op,width,a,raw,got,want)
                        shift_count+=1
    print(f"{shift_count:,} independent shift cases passed.")
    arithmetic_count=count
    def condition_flags(bits):
        return 0x202 | sum(((bits >> i)&1) << bit for i,bit in enumerate((0,2,4,6,7,11)))
    def expected_condition(name,f):
        c,p,z,s,o = (bool(f & (1<<bit)) for bit in (0,2,6,7,11))
        return int({"o":o,"no":not o,"b":c,"ae":not c,"e":z,"ne":not z,
                    "be":c or z,"a":not c and not z,"s":s,"ns":not s,
                    "p":p,"np":not p,"l":s!=o,"ge":s==o,
                    "le":z or s!=o,"g":not z and s==o}[name])
    for condition in CONDITIONS:
        code=tokens("set"+condition,8)
        for bits in range(64):
            f=condition_flags(bits)
            got=interpret(code,0xabcdff99,0,8,f,16)
            assert got==(0xabcdff00|expected_condition(condition,f),f,16)
            count+=1
    for width in (8,16,32):
        code=tokens("write_if_z",width)
        for taken in (False,True):
            a,b=0xabcd1234,0x87654321
            f=0x202|(64 if taken else 0)
            mask=(1<<width)-1
            got=interpret(code,a,b,width,f,ARITHMETIC_MASK)
            expected=(a & ~mask)|(b&mask) if taken else a
            assert got==(expected,f,ARITHMETIC_MASK)
            count+=1
    print(f"{arithmetic_count:,} arithmetic + {count-arithmetic_count:,} conditional cases passed.")
    print("Rust compilation is a separate check.")
