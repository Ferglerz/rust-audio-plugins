"""Extract the manufacturer's vector cutting marks from Rev C (16 Apr 2018)."""
import sys
from pypdf import PdfReader

marks = []
for args, op in PdfReader(sys.argv[1]).pages[0].get_contents().operations:
    if op == b'cm':
        x, y = map(float,args[4:6])
    if op == b'l' and args[1]==0 and (55 < x < 57 or 442 < x < 444) and y>330:
        marks.append((int(x>400), -y, float(args[0])))
marks.sort()
assert len(marks)==82 and marks[2][2]==432
for _, _, length in marks[13:77]:
    print(f'{length*25.4/72:.3f},',end=' ')
print()
