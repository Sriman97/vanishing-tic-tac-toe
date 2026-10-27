#!/usr/bin/env python3
"""Build web/standalone.html (scripts inlined, wasm base64-embedded) from web/index.html."""
import base64, pathlib

w = pathlib.Path(__file__).parent / "web"


def inline_js(name):
    return "<script>" + (w / name).read_text().replace("</script", "<\\/script") + "</script>"


b64 = base64.b64encode((w / "vanishing_ttt.wasm").read_bytes()).decode()
loader = (
    f'<script>const bin=atob("{b64}");const bytes=new Uint8Array(bin.length);'
    "for(let i=0;i<bin.length;i++)bytes[i]=bin.charCodeAt(i);"
    "load(URL.createObjectURL(new Blob([bytes],{type:'application/wasm'})));</script>"
)

html = (w / "index.html").read_text()
for old, new in [
    ('<script src="mq_js_bundle.js"></script>', inline_js("mq_js_bundle.js")),
    ('<script src="online.js"></script>', inline_js("online.js")),
    ('<script>load("vanishing_ttt.wasm");</script>', loader),
]:
    assert html.count(old) == 1, old
    html = html.replace(old, new)

(w / "standalone.html").write_text(html)
print("wrote", w / "standalone.html", len(html), "bytes")
