#!/usr/bin/env python3
"""Build web/standalone.html (JS bundle inlined, wasm base64-embedded) from web/*."""
import base64, pathlib
w = pathlib.Path(__file__).parent / "web"
js = (w / "mq_js_bundle.js").read_text().replace("</script", "<\\/script")
b64 = base64.b64encode((w / "vanishing_ttt.wasm").read_bytes()).decode()
html = f"""<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>Vanishing Tic Tac Toe</title>
<meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no">
<style>html,body,canvas{{margin:0;padding:0;width:100%;height:100%;overflow:hidden;background:#1a1a2e;touch-action:none}}canvas{{display:block}}</style>
</head><body><canvas id="glcanvas" tabindex="1"></canvas>
<script>{js}</script>
<script>
const b64="{b64}";
const bin=atob(b64);const bytes=new Uint8Array(bin.length);
for(let i=0;i<bin.length;i++)bytes[i]=bin.charCodeAt(i);
load(URL.createObjectURL(new Blob([bytes],{{type:'application/wasm'}})));
</script></body></html>"""
(w / "standalone.html").write_text(html)
print("wrote", w / "standalone.html", len(html), "bytes")
