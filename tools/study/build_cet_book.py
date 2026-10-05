import sys,json,re,csv,unicodedata,hashlib
from collections import defaultdict
from pathlib import Path
repo=Path(__file__).resolve().parents[2]
root=Path(sys.argv[1]).resolve()
full="--all" in sys.argv
heads=json.loads((root/"headwords.json").read_text(encoding="utf-8"))
gloss={}
for line in (repo/"assets/glossary/glossary-zh.tsv").read_text(encoding="utf-8").splitlines():
    if not line or line.startswith("#"):continue
    columns=line.split("\t")
    gloss[columns[0].lower()]=columns[1:3]
reverse=defaultdict(set)
for line in (repo/"assets/glossary/glossary-en.tsv").read_text(encoding="utf-8").splitlines():
    if not line or line.startswith("#"):continue
    columns=line.split("\t")
    if not 2<=len(columns[0])<=8:continue
    for sense in columns[1:3]:
        word=re.sub(r"^(?:n|v|adj|adv|pron|prep|conj|int|num|art|aux|vt|vi)\.\s*","",sense).split("|")[0].strip().lower()
        reverse[word].add(columns[0])
levels={}
for line in (repo/"assets/levels/levels-en.tsv").read_text(encoding="utf-8").splitlines():
    if line and not line.startswith("#") and "\t" in line:
        word,level=line.split("\t",1);levels[word.lower()]=level
def forms(raw):
    raw=re.sub(r"[123]$","",raw).strip()
    raw={"accordingto":"according to","oughtto":"ought to","babyboom":"baby boom","hotdog":"hot dog","jetlag":"jet lag","knowhow":"know-how"}.get(raw,raw)
    if raw.startswith("coup("): return ["coup","coup d'état"]
    if raw=="apartment/apt.":return ["apartment","apt."]
    if "/" in raw:
        first,other=raw.split("/",1)
        options=forms(first)
        options += [first[:-len(other[1:])]+other[1:]] if other.startswith("-") else forms(other)
        return options
    optional=re.search(r"\(([a-z]+)\)",raw)
    if optional:
        return forms(raw[:optional.start()]+raw[optional.end():])+forms(raw[:optional.start()]+optional[1]+raw[optional.end():])
    return [raw]
entries={}
trace=[]
for h in heads:
    if not h["cet6"] and not full:continue
    for word in forms(h["headword"]):
        word=unicodedata.normalize("NFKC",word).lower()
        if not re.fullmatch(r"[a-zé]+(?:[- '.][a-zé]+)*\.?",word):
            raise ValueError(f"unresolved headword {h}")
        senses=gloss.get(word,[])
        meaning="；".join(re.sub(r"^(?:n|v|adj|adv|pron|prep|conj|int|num|art|aux|vt|vi)\.\s*","",x) for x in senses)
        pos=re.match(r"^([a-z]+)\.\s",senses[0]).group(1)+"." if senses and re.match(r"^([a-z]+)\.\s",senses[0]) else ""
        mappings=sorted(reverse[word],key=lambda v:(len(v),v))[:8]
        entries[word]={"english":word,"meaning":meaning,"pos":pos,"example":"","level":levels.get(word,""),"tags":("CET6;NEEA-2016-full;meaning-Qingjian-LLM;"+("NEEA-2016-star" if h["cet6"] else "NEEA-2016-base")) if full else "CET6;NEEA-2016-star;meaning-Qingjian-LLM","input_keys":";".join(mappings),"equivalents":""}
        trace.append({"word":word,"official_headword":h["headword"],"pdf_page":h["pdf_page"],"official_star":h["cet6"]})
out=repo/"assets/study/cet"
out.mkdir(exist_ok=True)
prefix="cet6-neea-2016-full" if full else "cet6-neea-2016-star"
file=out/(prefix+".csv")
with file.open("w",encoding="utf-8-sig",newline="") as f:
    w=csv.DictWriter(f,fieldnames=list(next(iter(entries.values()))))
    w.writeheader();w.writerows(entries[k] for k in sorted(entries))
print("official star heads",sum(x["cet6"] for x in heads),"importable",len(entries),"meaning",sum(bool(e["meaning"]) for e in entries.values()),"mapped",sum(bool(e["input_keys"]) for e in entries.values()),"level",sum(bool(e["level"]) for e in entries.values()))
(out/("full-headword-provenance.json" if full else "headword-provenance.json")).write_text(json.dumps(trace,ensure_ascii=False,indent=2),encoding="utf-8")
manifest={"official_source":"https://cet.neea.edu.cn/res/Home/1704/55b02330ac17274664f06d9d3db8249d.pdf","official_pdf_sha256":hashlib.sha256((root/"cet-syllabus-2016.pdf").read_bytes()).hexdigest(),"checked_at":"2026-10-05","official_pdf_pages":[21,149],"official_star_heads":sum(x["cet6"] for x in heads),"entries":len(entries),"meaning_count":sum(bool(e["meaning"]) for e in entries.values()),"input_mapping_count":sum(bool(e["input_keys"]) for e in entries.values()),"cefr_count":sum(bool(e["level"]) for e in entries.values()),"csv_sha256":hashlib.sha256(file.read_bytes()).hexdigest(),"scope":("All extracted four/six-level primary headwords, expanded spelling variants and merged homographs; excludes separately printed derivative column. Official stated 5418 is not the deduplicated CSV count." if full else "Six-level starred headwords only; expanded spelling variants, not the entire four/six-level wordlist."),"extracted_primary_rows":len(heads),"official_stated_primary_count":5418,"meaning_source":"Qingjian GPL-3.0-or-later offline LLM glossary; not official definitions.","level_source":"Bundled CEFR-J/Octanove tags; separate from official CET level.","official_licence":"No explicit open data licence identified. Official factual headword and star metadata only; no original PDF, explanatory paragraphs or exam samples redistributed."}
(out/("full-source-manifest.json" if full else "source-manifest.json")).write_text(json.dumps(manifest,ensure_ascii=False,indent=2),encoding="utf-8")