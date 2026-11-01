"""Verify the authored PDF, including Chromium's embedded Type3 CFF glyphs."""
from pathlib import Path
import json
import sys
from pypdf import PdfReader

ROOT = Path(__file__).resolve().parents[2]
pdf = ROOT / "docs/05_开源与参赛/release-preparation/output/pdf/超级工厂_作品介绍_开发候选.pdf"
reader = PdfReader(pdf)
pages, fonts = [], {}
for index, page in enumerate(reader.pages):
    text = page.extract_text()
    pages.append({"page": index + 1, "extracted_characters": len(text),
                  "chinese_present": any("\u4e00" <= x <= "\u9fff" for x in text),
                  "replacement_glyph": "\ufffd" in text})
    for ref in page["/Resources"].get("/Font", {}).values():
        font = ref.get_object()
        if str(font.get("/Subtype")) == "/Type3":
            procedures = font.get("/CharProcs", {})
            procedures = procedures.get_object() if hasattr(procedures, "get_object") else procedures
            embedded = bool(procedures) and all(bool(p.get_object().get_data()) for p in procedures.values())
        else:
            base = font["/DescendantFonts"][0].get_object() if "/DescendantFonts" in font else font
            descriptor = base.get("/FontDescriptor")
            descriptor = descriptor.get_object() if descriptor else {}
            embedded = any(k in descriptor for k in ("/FontFile", "/FontFile2", "/FontFile3"))
        fonts[str(ref.idnum)] = {"subtype": str(font.get("/Subtype")), "self_contained_glyph_programs": embedded}
assert len(pages) == 10 and all(p["chinese_present"] and not p["replacement_glyph"] for p in pages)
assert fonts and all(font["self_contained_glyph_programs"] for font in fonts.values())
result = {"pdf_pages": len(pages), "pages": pages, "font_objects": len(fonts), "all_fonts_self_contained": True,
          "font_encoding": "SourceHan OFL CFF rendered as Type3 embedded glyph programs; no external font required",
          "method_correction": "Original FontFile-only inspection did not cover Type3; verified actual CharProcs streams instead of treating this PDF as missing fonts.",
          "actual_video_path": None, "product_runtime_verified_by_this_document_check": False}
output = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / ".work/whole-packages/window2-release-preparation/pdf-readback.json"
output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf8")
print(json.dumps({key: value for key, value in result.items() if key != "pages"}, ensure_ascii=False))
