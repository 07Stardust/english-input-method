import sys,json,re,unicodedata
from pathlib import Path
from pypdf import PdfReader
root=Path(sys.argv[1]).resolve()
heads=[h for h in json.loads((root/"headwords.json").read_text(encoding="utf-8")) if h["cet6"]]
pages=[page.extract_text() for page in PdfReader(root/"cet-syllabus-2016.pdf").pages]
def norm(s):return re.sub(r"[\s123]","",unicodedata.normalize("NFKC",s.replace("Ｇ","-").replace("\U001001b3","'")))
errors=[]
stars=0
for i,text in enumerate(pages[20:149],20):
    for raw in text.splitlines():
        if "★" not in raw:continue
        stars+=1
        main=norm(raw.replace("★","").strip().split()[0])
        matches=[h for h in heads if h["pdf_page"]==i+1 and norm(h["headword"]).startswith(main)]
        if not matches:errors.append((i+1,raw))
print("independent text-star count",stars,"geometry-star count",len(heads),"mismatches",errors)
assert stars==1263==len(heads) and not errors