"use strict";
// Peer-to-peer online play. Two browsers open a WebRTC data channel by swapping
// two copy-and-paste codes (no game server). The Rust game talks to this file
// through four functions (vttt_status, vttt_role, vttt_recv, vttt_send).
(function () {
  const ICE = [{ urls: "stun:stun.l.google.com:19302" }];
  const GATHER_TIMEOUT_MS = 4000;

  // status: 0 = local play, 1 = connecting, 2 = connected, 3 = connection lost.
  // role: 1 = X (host), 2 = O (guest). Messages are cell numbers 0-8, or 9 for a rematch.
  const net = { status: 0, role: 0, inbox: [] };
  let pc = null;
  let dc = null;
  let hosting = false;

  miniquad_add_plugin({
    name: "vttt_net",
    version: 1,
    register_plugin: function (imp) {
      imp.env.vttt_status = () => net.status;
      imp.env.vttt_role = () => net.role;
      imp.env.vttt_recv = () => (net.inbox.length ? net.inbox.shift() : -1);
      imp.env.vttt_send = (v) => {
        if (dc && dc.readyState === "open") dc.send(String(v));
      };
    },
  });

  // ---- codes -------------------------------------------------------------
  // A code is "V1" + base64url of the (deflated, when supported) session description.
  const hasDeflate = typeof CompressionStream === "function" && typeof DecompressionStream === "function";

  function toB64(bytes) {
    let s = "";
    for (let i = 0; i < bytes.length; i++) s += String.fromCharCode(bytes[i]);
    return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  }
  function fromB64(str) {
    const b = atob(str.replace(/-/g, "+").replace(/_/g, "/"));
    const out = new Uint8Array(b.length);
    for (let i = 0; i < b.length; i++) out[i] = b.charCodeAt(i);
    return out;
  }
  async function pipe(bytes, stream) {
    const buf = await new Response(new Blob([bytes]).stream().pipeThrough(stream)).arrayBuffer();
    return new Uint8Array(buf);
  }

  async function pack(desc) {
    const raw = new TextEncoder().encode(JSON.stringify({ t: desc.type, s: desc.sdp }));
    if (hasDeflate) return "V1z" + toB64(await pipe(raw, new CompressionStream("deflate-raw")));
    return "V1r" + toB64(raw);
  }
  async function unpack(code) {
    const c = code.replace(/\s+/g, "");
    if (c.slice(0, 2) !== "V1") throw new Error("That is not a code from this game.");
    let raw = fromB64(c.slice(3));
    if (c[2] === "z") {
      if (!hasDeflate) throw new Error("This browser cannot read that code. Ask your friend to use a newer browser.");
      raw = await pipe(raw, new DecompressionStream("deflate-raw"));
    }
    const o = JSON.parse(new TextDecoder().decode(raw));
    return { type: o.t, sdp: o.s };
  }

  // ---- connection --------------------------------------------------------
  function waitForCandidates(conn) {
    return new Promise((resolve) => {
      if (conn.iceGatheringState === "complete") return resolve();
      const done = () => {
        conn.removeEventListener("icegatheringstatechange", check);
        resolve();
      };
      const check = () => conn.iceGatheringState === "complete" && done();
      conn.addEventListener("icegatheringstatechange", check);
      setTimeout(done, GATHER_TIMEOUT_MS);
    });
  }

  function teardown() {
    const old = pc;
    pc = null;
    dc = null;
    if (old) {
      old.onconnectionstatechange = null;
      old.close();
    }
    net.status = 0;
    net.role = 0;
    net.inbox.length = 0;
  }

  function onLost(why) {
    if (net.status === 2) {
      net.status = 3;
      show("lost", why);
    } else if (net.status === 1) {
      net.status = 0;
      show("failed", why);
    }
  }

  function newConnection() {
    teardown();
    net.status = 1;
    pc = new RTCPeerConnection({ iceServers: ICE });
    pc.onconnectionstatechange = () => {
      if (pc && (pc.connectionState === "failed" || pc.connectionState === "closed")) {
        onLost(pc.connectionState === "failed" ? "The connection failed." : "Your friend left the game.");
      } else if (pc && pc.connectionState === "disconnected") {
        setTimeout(() => {
          if (pc && pc.connectionState === "disconnected") onLost("The connection dropped.");
        }, 5000);
      }
    };
    return pc;
  }

  function attachChannel(channel) {
    dc = channel;
    dc.onopen = () => {
      net.inbox.length = 0;
      net.role = hosting ? 1 : 2;
      net.status = 2;
      hide();
    };
    dc.onmessage = (e) => {
      const v = parseInt(e.data, 10);
      if (v >= 0 && v <= 9) net.inbox.push(v);
    };
    dc.onclose = () => onLost("Your friend left the game.");
  }

  async function startHost() {
    hosting = true;
    const conn = newConnection();
    attachChannel(conn.createDataChannel("game"));
    await conn.setLocalDescription(await conn.createOffer());
    await waitForCandidates(conn);
    return pack(conn.localDescription);
  }

  async function startJoin(offerCode) {
    hosting = false;
    const offer = await unpack(offerCode);
    const conn = newConnection();
    conn.ondatachannel = (e) => attachChannel(e.channel);
    await conn.setRemoteDescription(offer);
    await conn.setLocalDescription(await conn.createAnswer());
    await waitForCandidates(conn);
    return pack(conn.localDescription);
  }

  async function finishHost(answerCode) {
    if (!pc) throw new Error("Start a new game first.");
    await pc.setRemoteDescription(await unpack(answerCode));
  }

  // ---- menu --------------------------------------------------------------
  const $ = (id) => document.getElementById(id);
  const views = ["home", "host", "join", "lost"];

  function show(view, message) {
    $("menu").hidden = false;
    $("menuBtn").hidden = true;
    for (const v of views) $("v-" + (v === "failed" ? "lost" : v)).hidden = v !== (view === "failed" ? "lost" : view);
    if (view === "lost" || view === "failed") $("lostMsg").textContent = message || "The connection ended.";
    if (view === "host" || view === "join") for (const e of $("v-" + view).querySelectorAll(".err")) e.textContent = "";
  }
  function hide() {
    $("menu").hidden = true;
    $("menuBtn").hidden = false;
  }
  function setError(view, err) {
    $("v-" + view).querySelector(".err").textContent = err && err.message ? err.message : String(err);
  }

  async function copyText(textarea, button) {
    textarea.focus();
    textarea.select();
    textarea.setSelectionRange(0, textarea.value.length);
    let ok = false;
    try {
      await navigator.clipboard.writeText(textarea.value);
      ok = true;
    } catch (_) {
      try {
        ok = document.execCommand("copy");
      } catch (_) {}
    }
    const label = button.dataset.label || button.textContent;
    button.dataset.label = label;
    button.textContent = ok ? "Copied" : "Press and hold to copy";
    setTimeout(() => (button.textContent = label), 1800);
  }

  async function shareText(text) {
    try {
      await navigator.share({ title: "Vanishing Tic Tac Toe", text });
    } catch (_) {}
  }

  function wire() {
    const canShare = typeof navigator.share === "function";
    for (const b of document.querySelectorAll(".share")) b.hidden = !canShare;

    $("btnLocal").onclick = () => {
      teardown();
      hide();
    };
    $("btnHost").onclick = async () => {
      show("host");
      $("hostOut").value = "Making your code...";
      $("hostGo").disabled = true;
      try {
        $("hostOut").value = await startHost();
        $("hostGo").disabled = false;
      } catch (err) {
        $("hostOut").value = "";
        setError("host", err);
      }
    };
    $("btnJoin").onclick = () => {
      show("join");
      $("joinIn").value = "";
      $("joinOut").value = "";
      $("joinOutBox").hidden = true;
    };
    $("hostCopy").onclick = () => copyText($("hostOut"), $("hostCopy"));
    $("hostShare").onclick = () => shareText($("hostOut").value);
    $("hostGo").onclick = async () => {
      try {
        await finishHost($("hostIn").value);
        $("hostGo").disabled = true;
        $("hostGo").textContent = "Connecting...";
        setTimeout(() => {
          $("hostGo").textContent = "Connect";
          $("hostGo").disabled = false;
        }, 15000);
      } catch (err) {
        setError("host", err);
      }
    };
    $("joinGo").onclick = async () => {
      $("joinGo").disabled = true;
      $("joinGo").textContent = "Making your reply...";
      try {
        $("joinOut").value = await startJoin($("joinIn").value);
        $("joinOutBox").hidden = false;
        $("joinOut").scrollIntoView({ block: "nearest" });
      } catch (err) {
        teardown();
        setError("join", err);
      }
      $("joinGo").disabled = false;
      $("joinGo").textContent = "Make reply code";
    };
    $("joinCopy").onclick = () => copyText($("joinOut"), $("joinCopy"));
    $("joinShare").onclick = () => shareText($("joinOut").value);
    $("lostLocal").onclick = $("btnLocal").onclick;
    $("lostAgain").onclick = () => show("home");
    for (const b of document.querySelectorAll(".back")) {
      b.onclick = () => {
        teardown();
        show("home");
      };
    }
    $("menuBtn").onclick = () => show("home");
    show("home");
  }

  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", wire);
  else wire();
})();
