// Claude Halo — Canvas Renderer (v2: morphing + entry/exit + compacting)
(function () {
  "use strict";

  // ── Utils ──────────────────────────────────────────────────────────
  function hexRgb(h) {
    var m = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(h);
    return m ? [parseInt(m[1], 16), parseInt(m[2], 16), parseInt(m[3], 16)] : [255, 255, 255];
  }
  function lerp(a, b, t) { return a + (b - a) * t; }
  function lerpRgb(a, b, t) {
    return [Math.round(lerp(a[0], b[0], t)), Math.round(lerp(a[1], b[1], t)), Math.round(lerp(a[2], b[2], t))];
  }
  function rgba(r, g, b, a) { return "rgba(" + r + "," + g + "," + b + "," + a + ")"; }
  function easeOut(t) { return 1 - Math.pow(1 - t, 3); }
  function easeInOut(t) { return t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2; }
  function easeOutBack(t) {
    var c1 = 1.70158;
    return 1 + (c1 + 1) * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2);
  }

  // ── Configs ────────────────────────────────────────────────────────
  var configs = {
    idle:    { color: "#aaaaaa", halo: "#cccccc", period: 6.0, dashes: [60, 30],
               ms: 0,   md: 0,    amin: 0.30, amax: 0.42, br: 0,   rp: 0,    rpperiod: 0 },
    thinking:{ color: "#ff8830", halo: "#ffdbb8", period: 2.4, dashes: [70, 35, 45, 30, 25, 20],
               ms: 0.6, md: 0.4,  amin: 0.45, amax: 0.90, br: 5.2, rp: 0,    rpperiod: 0 },
    executing:{color: "#3399ff", halo: "#bbddff", period: 1.3, dashes: [50, 25, 20, 20, 35, 25, 25, 22],
               ms: 1.2, md: 0.28, amin: 0.60, amax: 0.90, br: 0,   rp: 0,    rpperiod: 0 },
    input_needed:{color: "#ee3333", halo: "#ffcccc", period: 2.8, dashes: [80, 50, 30, 25],
               ms: 1.8, md: 0.5,  amin: 0.52, amax: 0.94, br: 2.0, rp: 0,    rpperiod: 0 },
    completed:{color: "#33cc55", halo: "#bbffcc", period: 5.0, dashes: [70, 35, 45, 30, 25, 20],
               ms: 0.5, md: 0.3,  amin: 0.38, amax: 0.84, br: 6.0, rp: 0,    rpperiod: 0 },
    compacting:{color: "#9944ff", halo: "#ddccff", period: 2.1, dashes: [35, 20, 35, 20, 35, 20],
               ms: 0.4, md: 0.25, amin: 0.38, amax: 0.80, br: 4.0, rp: 0.12, rpperiod: 1.6 },
  };

  // Public interface (app.js reads/writes this)
  window.__haloState = "idle";
  // Called by app.js to trigger animated close
  window.__haloTriggerExit = null;

  var canvas = document.getElementById("halo");
  var ctx = canvas.getContext("2d");

  // ── Render state ───────────────────────────────────────────────────
  var time = 0, lastT = null;
  var rafId = null;

  var currentState = "idle", currentCfg = configs.idle;
  var morphFrom = null, morphTarget = "idle";
  var morphStart = 0, morphActive = false;
  var MORPH_DURATION = 420; // ms

  // Pulse settle-in (compacting)
  var pulseSettleStart = 0;
  var PULSE_SETTLE_DELAY  = 0.20; // 200ms pause before ramp
  var PULSE_RAMP_DURATION = 0.50; // 500ms gentle ramp

  // Lifecycle: 'entering' → 'active' → 'exiting'
  var lifecycle = "entering", entryStart = null, ENTRY_DURATION = 550;
  var exitStart = 0, EXIT_DURATION = 250;
  // Callback after exit animation finishes (set by app.js)
  var onExitComplete = null;

  // ── Resize ─────────────────────────────────────────────────────────
  function resize() {
    var dpr = window.devicePixelRatio || 1;
    var w = window.innerWidth;
    var h = window.innerHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }
  resize();
  window.addEventListener("resize", resize);

  // ── Morph: interpolate dash arrays across different segment counts ─
  function morphDashArrays(a, b, t) {
    var nA = a.length / 2, nB = b.length / 2;
    var nFloat = lerp(nA, nB, t);
    var nDraw = Math.max(1, Math.ceil(nFloat));
    var result = [];
    var totalA = a.reduce(function (s, v) { return s + v; }, 0);
    var totalB = b.reduce(function (s, v) { return s + v; }, 0);
    var totalTarget = lerp(totalA, totalB, t);

    for (var i = 0; i < nDraw; i++) {
      var angleFrac = i / nDraw;
      var segA = angleFrac * nA, idxA = Math.floor(segA) % nA;
      var dashA = a[idxA * 2], gapA = a[idxA * 2 + 1];
      var segB = angleFrac * nB, idxB = Math.floor(segB) % nB;
      var dashB = b[idxB * 2], gapB = b[idxB * 2 + 1];
      var dashLen = lerp(dashA, dashB, t), gapLen = lerp(gapA, gapB, t);
      var birthScale = nDraw <= nFloat ? 1 : 1 - (nDraw - nFloat);
      birthScale = Math.max(0.02, birthScale);
      dashLen *= birthScale;
      gapLen *= birthScale;
      result.push(dashLen, gapLen);
    }

    var curTotal = result.reduce(function (s, v) { return s + v; }, 0);
    if (curTotal > 0) {
      var scale = totalTarget / curTotal;
      for (var j = 0; j < result.length; j++) result[j] *= scale;
    }
    return result;
  }

  function morphConfig(cfgA, cfgB, t) {
    return {
      color:  lerpRgb(hexRgb(cfgA.color), hexRgb(cfgB.color), t),
      halo:   lerpRgb(hexRgb(cfgA.halo),  hexRgb(cfgB.halo),  t),
      period: lerp(cfgA.period, cfgB.period, t),
      ms:     lerp(cfgA.ms, cfgB.ms, t),
      md:     lerp(cfgA.md, cfgB.md, t),
      amin:   lerp(cfgA.amin, cfgB.amin, t),
      amax:   lerp(cfgA.amax, cfgB.amax, t),
      br:     lerp(cfgA.br, cfgB.br, t),
      rp:     lerp(cfgA.rp, cfgB.rp, t),
      rpperiod: lerp(cfgA.rpperiod, cfgB.rpperiod, t),
      dashes: morphDashArrays(cfgA.dashes, cfgB.dashes, t),
    };
  }

  // ── Draw single ring ──────────────────────────────────────────────
  function drawRing(cx, cy, R, cfg, alphaMul, timeOffset) {
    if (alphaMul <= 0.001) return;
    var t = time + (timeOffset || 0);
    var mainRgb = Array.isArray(cfg.color) ? cfg.color : hexRgb(cfg.color);
    var haloRgb = Array.isArray(cfg.halo) ? cfg.halo : hexRgb(cfg.halo);

    // Breathing alpha
    var breathe = 0;
    if (cfg.br > 0) {
      var ph = (t % cfg.br) / cfg.br;
      breathe = Math.max(0, Math.sin(ph * Math.PI * 2));
    }
    var alpha = (cfg.br > 0 ? cfg.amin + (cfg.amax - cfg.amin) * breathe : cfg.amax) * alphaMul;

    var offset = ((t / cfg.period) * Math.PI * 2) % (Math.PI * 2);
    var rawSum = cfg.dashes.reduce(function (a, b) { return a + b; }, 0);

    // ── Anti-overlap: gap minimum = halo stroke angular extent + 20% ─
    var haloLw = Math.max(R * 0.28, 1.5);
    var haloAngExt = haloLw / R;
    var minGapAngle = haloAngExt * 1.20;

    // Pass 1: compute raw angles, protect gaps
    var rawAngles = [];
    for (var i = 0; i < cfg.dashes.length; i++) {
      var morph = Math.sin(t * cfg.ms + i * 2.1);
      var ang = (cfg.dashes[i] / rawSum) * Math.PI * 2;
      if (i % 2 === 0) {
        // Dash segment
        ang *= 1 + cfg.md * morph;
        ang = Math.max(0.02, ang);
      } else {
        // Gap segment — enforce minimum separation
        ang *= 1 - cfg.md * 0.5 * morph;
        ang = Math.max(minGapAngle, ang);
      }
      rawAngles.push(ang);
    }

    // Pass 2: normalize — scale dashes only, keep gaps at their minimum
    var dashTotal = 0, gapTotal = 0;
    for (var j = 0; j < rawAngles.length; j++) {
      if (j % 2 === 0) dashTotal += rawAngles[j];
      else gapTotal += rawAngles[j];
    }
    var dashTarget = Math.PI * 2 - gapTotal;
    var dashScale = dashTotal > 0.001 ? dashTarget / dashTotal : 1;

    var cumul = 0;
    for (var k = 0; k < rawAngles.length; k++) {
      var ang = k % 2 === 0 ? rawAngles[k] * dashScale : rawAngles[k];
      var a0 = cumul - offset, a1 = a0 + ang;
      cumul += ang;
      if (k % 2 === 1) continue; // skip gap segments

      // Outer halo
      ctx.strokeStyle = rgba(haloRgb[0], haloRgb[1], haloRgb[2], alpha * 0.55);
      ctx.lineWidth = haloLw;
      ctx.lineCap = "round";
      ctx.beginPath();
      ctx.arc(cx, cy, R, a0, a1, false);
      ctx.stroke();

      // Mid layer
      var mid = lerpRgb(haloRgb, mainRgb, 0.5);
      ctx.strokeStyle = rgba(mid[0], mid[1], mid[2], alpha * 0.85);
      ctx.lineWidth = Math.max(R * 0.16, 1);
      ctx.lineCap = "round";
      ctx.beginPath();
      ctx.arc(cx, cy, R, a0, a1, false);
      ctx.stroke();

      // Core line
      ctx.strokeStyle = rgba(mainRgb[0], mainRgb[1], mainRgb[2], alpha);
      ctx.lineWidth = Math.max(R * 0.10, 1);
      ctx.lineCap = "round";
      ctx.beginPath();
      ctx.arc(cx, cy, R, a0, a1, false);
      ctx.stroke();
    }
  }

  // ── Glow bridge: full-ring soft overlay at morph midpoint ─────────
  function drawGlowBridge(cx, cy, R, cfgA, cfgB, t) {
    var glowAlpha = Math.sin(t * Math.PI) * 0.32;
    if (glowAlpha < 0.005) return;
    var rgbA = Array.isArray(cfgA.color) ? cfgA.color : hexRgb(cfgA.color);
    var rgbB = Array.isArray(cfgB.color) ? cfgB.color : hexRgb(cfgB.color);
    var mid = lerpRgb(rgbA, rgbB, t);

    // Wide outer glow
    ctx.strokeStyle = rgba(mid[0], mid[1], mid[2], glowAlpha * 0.22);
    ctx.lineWidth = Math.max(R * 0.50, 3.5);
    ctx.lineCap = "round";
    ctx.beginPath();
    ctx.arc(cx, cy, R, 0, Math.PI * 2, false);
    ctx.stroke();

    // Tighter inner glow
    ctx.strokeStyle = rgba(mid[0], mid[1], mid[2], glowAlpha * 0.40);
    ctx.lineWidth = Math.max(R * 0.22, 1.8);
    ctx.lineCap = "round";
    ctx.beginPath();
    ctx.arc(cx, cy, R, 0, Math.PI * 2, false);
    ctx.stroke();
  }

  // ── Phase sync: align rotation phases of old/new configs ──────────
  function phaseSyncTimeOffset(cfgOld, cfgNew, tNow) {
    var oldPhase = (tNow / cfgOld.period) % 1;
    var newPhase = (tNow / cfgNew.period) % 1;
    return (oldPhase - newPhase) * cfgNew.period;
  }

  // ── Radius pulse (compacting) — ramps in after morph settles ──────
  function radiusPulseFactor(cfg, tNow) {
    if (!cfg.rp || cfg.rpperiod <= 0) return 1;
    if (pulseSettleStart <= 0) return 1;
    var elapsed = tNow - pulseSettleStart;
    if (elapsed < 0) return 1;
    // 200ms pause, then 500ms gentle ramp
    var ramp = Math.min(1, Math.max(0, elapsed - PULSE_SETTLE_DELAY) / PULSE_RAMP_DURATION);
    var effectiveRp = cfg.rp * easeOut(ramp);
    if (effectiveRp <= 0.001) return 1;
    var phase = (tNow % cfg.rpperiod) / cfg.rpperiod;
    return 1 + effectiveRp * Math.sin(phase * Math.PI * 2);
  }

  // ── RAF management (prevents double-scheduling) ───────────────────
  function scheduleFrame() {
    if (rafId !== null) return;
    rafId = requestAnimationFrame(frame);
  }

  function stopLoop() {
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
  }

  // ── Main render loop ──────────────────────────────────────────────
  function frame(ts) {
    rafId = null;

    if (!lastT) lastT = ts;
    var dt = (ts - lastT) / 1000;
    if (dt <= 0) dt = 0.016;
    else if (dt > 0.1) dt = 0.1;
    lastT = ts;
    time += dt;

    var w = window.innerWidth;
    var h = window.innerHeight;
    var cx = Math.floor(w / 2);
    var cy = Math.floor(h / 2);
    var baseR = Math.min(w, h) * 0.40;

    // ── Entry animation ────────────────────────────────────────────
    var entryScale = 1, entryAlpha = 1;
    if (lifecycle === "entering") {
      if (!entryStart) entryStart = ts;
      var raw = Math.min((ts - entryStart) / ENTRY_DURATION, 1);
      entryScale = 0.2 + 0.8 * easeOutBack(raw);
      entryAlpha = easeOut(raw);
      if (raw >= 1) { lifecycle = "active"; }
    }

    // ── Exit animation ─────────────────────────────────────────────
    var exitScale = 1, exitAlpha = 1;
    if (lifecycle === "exiting") {
      var rawE = Math.min((ts - exitStart) / EXIT_DURATION, 1);
      var tE = 1 - Math.pow(1 - rawE, 2); // ease-in
      exitScale = 1 - tE;
      exitAlpha = 1 - tE;
      if (rawE >= 1) {
        lifecycle = "closed";
        stopLoop();
        // Notify app.js to actually close the window
        if (onExitComplete) { onExitComplete(); }
        return;
      }
    }

    var ringScale = entryScale * exitScale;
    var ringAlpha = entryAlpha * exitAlpha;

    // ── Active config (for radius pulse calculation) ───────────────
    var activeCfg = morphActive
      ? morphConfig(morphFrom, configs[morphTarget],
          Math.min((ts - morphStart) / MORPH_DURATION, 1))
      : currentCfg;

    var rpFactor = radiusPulseFactor(activeCfg, time);
    var RR = baseR * ringScale * rpFactor;

    ctx.clearRect(0, 0, w, h);

    // ── Detect state change from app.js ────────────────────────────
    var wanted = lifecycle === "active" ? (window.__haloState || "idle") : currentState;
    if (lifecycle === "active" && wanted !== morphTarget && wanted !== currentState) {
      // Kill any in-progress pulse
      pulseSettleStart = 0;
      morphFrom = morphActive
        ? morphConfig(configs[morphTarget], configs[wanted],
            Math.min((ts - morphStart) / MORPH_DURATION, 1))
        : currentCfg;
      morphTarget = wanted;
      morphStart = ts;
      morphActive = true;
    }

    // ── Render ─────────────────────────────────────────────────────
    if (lifecycle !== "active") {
      drawRing(cx, cy, RR, currentCfg, ringAlpha, 0);
    } else if (morphActive) {
      var elapsed = ts - morphStart;
      var raw = Math.min(elapsed / MORPH_DURATION, 1);
      var tEased = easeInOut(raw);
      var targetCfg = configs[morphTarget];
      var interpCfg = morphConfig(morphFrom, targetCfg, tEased);
      var timeOff = phaseSyncTimeOffset(morphFrom, targetCfg, time);

      // Single morphing ring
      drawRing(cx, cy, RR, interpCfg, ringAlpha, timeOff);
      // Glow bridge at midpoint
      drawGlowBridge(cx, cy, RR, morphFrom, targetCfg, tEased);

      if (raw >= 1) {
        morphActive = false;
        currentState = morphTarget;
        currentCfg = targetCfg;
        morphFrom = null;
        // If target has radius pulse, schedule settle-in after morph
        if (targetCfg.rp > 0) {
          pulseSettleStart = time + 0.06;
        }
      }
    } else {
      drawRing(cx, cy, RR, currentCfg, ringAlpha, 0);
    }

    if (lifecycle !== "closed") scheduleFrame();
  }

  // ── Trigger exit animation (called by app.js) ─────────────────────
  window.__haloTriggerExit = function (onComplete) {
    if (lifecycle === "exiting" || lifecycle === "closed") return;
    lifecycle = "exiting";
    exitStart = performance.now();
    onExitComplete = onComplete || null;
    // Ensure loop is running (it may have been stopped)
    scheduleFrame();
  };

  // ── Kick off ─────────────────────────────────────────────────────
  scheduleFrame();
})();
