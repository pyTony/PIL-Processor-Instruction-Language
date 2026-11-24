"""Repack a .tar.gz as a ustar archive owned by root, gzip'ed, so that
MCC Linux 1.0's `cpio -H ustar` reads it (GNU tar headers it does not).

    python tools/mcc/ustar_repack.py IN.tar.gz OUT.tgz
"""
import gzip
import io
import sys
import tarfile


def main(argv):
    if len(argv) != 3:
        sys.exit("usage: ustar_repack.py IN.tar.gz OUT.tgz")
    buf = io.BytesIO()
    with tarfile.open(argv[1]) as src, \
            tarfile.open(fileobj=buf, mode="w", format=tarfile.USTAR_FORMAT) as out:
        for m in src.getmembers():
            m.uid = m.gid = 0
            m.uname = m.gname = "root"
            out.addfile(m, src.extractfile(m) if m.isfile() else None)
    with open(argv[2], "wb") as f:
        f.write(gzip.compress(buf.getvalue(), mtime=0))


if __name__ == "__main__":
    main(sys.argv)
