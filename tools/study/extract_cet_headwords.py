"""从官方 CET 2016 PDF 的坐标列提取词目与六级星标，不复制样卷或正文。"""
import sys,json,unicodedata
from collections import defaultdict
from pathlib import Path
import pdfplumber
root=Path(sys.argv[1]).resolve()
records=[]
with pdfplumber.open(root/"cet-syllabus-2016.pdf") as pdf:
    for number in range(20,149):
        lines=defaultdict(list)
        for c in pdf.pages[number].chars:
            lines[round(c["matrix"][5],1)].append(c)
        grouped=[]
        for y,cs in sorted(lines.items(),reverse=True):
            if grouped and grouped[-1][0]-y<6.0:
                grouped[-1][1].extend(cs)
            else:
                grouped.append((y,cs))
        for baseline,chars in grouped:
            chars=sorted(chars,key=lambda c:c["x0"])
            data=[c for c in chars if c["x0"]>=80 and c["top"]>70]
            if not data or not 80<=data[0]["x0"]<165 or not data[0]["text"].isascii() or not data[0]["text"].isalpha():
                continue
            primary=[]
            last=None
            for c in data:
                gap=c["x0"]-last["x1"] if last else 0
                if c["x0"]>=165 and gap>=1.5:
                    break
                if last and gap>=1.4:
                    primary.append(" ")
                primary.append(c["text"])
                last=c
            primary=unicodedata.normalize("NFKC","".join(primary).replace("Ｇ","-").replace("\U001001b3","'")).strip()
            records.append({"headword":primary,"cet6":any(c["text"]=="★" for c in chars),"pdf_page":number+1,"baseline":baseline})
print("heads",len(records),"starred",sum(r["cet6"] for r in records))
print("first",records[:5])
print("multiword",[r["headword"] for r in records if " " in r["headword"]][:25])
print("last",records[-3:])
(root/"headwords.json").write_text(json.dumps(records,ensure_ascii=False,indent=2),encoding="utf-8")