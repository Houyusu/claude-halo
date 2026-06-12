// Claude Halo — App Logic
(function () {
  var canvas = document.getElementById("halo");
  var menu = document.getElementById("menu");
  var summary = document.getElementById("summary");
  var summaryStatus = document.getElementById("summary-status");
  var summaryInfo = document.getElementById("summary-info");
  var passthroughHint = document.getElementById("passthrough-hint");
  var passthroughItem = menu.querySelector('[data-state="passthrough"]');

  var currentState = "idle";
  var passthroughEnabled = true;
  var passthroughTimeout = null;

  var hasTauri = typeof window.__TAURI_INTERNALS__ !== "undefined";
  var invoke = hasTauri ? window.__TAURI_INTERNALS__.invoke : null;

  // ─── State Management ───────────────────────────────────────────
  function setState(state) {
    currentState = state;
    window.__haloState = state;
    updateSummary();
  }

  var labels = {
    idle: "待命",
    thinking: "思考中",
    executing: "执行中",
    input_needed: "等待输入",
    completed: "已完成",
  };
  var infos = {
    idle: "Claude 未运行",
    thinking: "Claude 正在思考…",
    executing: "正在执行工具…",
    input_needed: "等待你的输入",
    completed: "任务完成",
  };

  function updateSummary() {
    summaryStatus.textContent = labels[currentState] || currentState;
    summaryStatus.className = "status " + currentState;
    summaryInfo.textContent = infos[currentState] || "";
  }

  // ─── Passthrough ────────────────────────────────────────────────
  function updatePassthroughUI() {
    passthroughItem.classList.toggle("on", passthroughEnabled);
    passthroughItem.classList.toggle("off", !passthroughEnabled);
    canvas.style.pointerEvents = passthroughEnabled ? "none" : "auto";
  }

  function flashPassthroughHint(msg) {
    passthroughHint.textContent = msg;
    passthroughHint.style.display = "block";
    setTimeout(function () {
      passthroughHint.style.display = "none";
    }, 2000);
  }

  function disablePassthroughTemporarily() {
    if (!hasTauri) return;
    clearTimeout(passthroughTimeout);
    invoke("set_passthrough", { enabled: false })
      .then(function () {
        passthroughEnabled = false;
        updatePassthroughUI();
        flashPassthroughHint("穿透已暂停 · 15秒后自动恢复 · Ctrl+Shift+F12 切换");
        passthroughTimeout = setTimeout(function () {
          enablePassthrough();
        }, 15000);
      })
      .catch(function (e) {
        console.error("set_passthrough failed:", e);
      });
  }

  function enablePassthrough() {
    if (!hasTauri) return;
    clearTimeout(passthroughTimeout);
    invoke("set_passthrough", { enabled: true })
      .then(function () {
        passthroughEnabled = true;
        updatePassthroughUI();
      })
      .catch(function (e) {
        console.error("set_passthrough failed:", e);
      });
  }

  function togglePassthrough() {
    if (passthroughEnabled) { disablePassthroughTemporarily(); }
    else { enablePassthrough(); }
  }

  // ─── Tauri IPC ──────────────────────────────────────────────────
  if (hasTauri) {
    // Polling fallback (300ms — catches state if event listener fails)
    setInterval(function () {
      invoke("get_state")
        .then(function (state) {
          if (state !== currentState) { setState(state); }
        })
        .catch(function () {});
    }, 300);

    // Tauri 2.x event listener
    try {
      var tauriEvent = (window.__TAURI__ && window.__TAURI__.event)
        || (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.event);
      if (tauriEvent) {
        tauriEvent.listen("state-changed", function (event) {
          setState(event.payload);
        });
        // Global shortcut from Rust backend — works regardless of focus
        tauriEvent.listen("toggle-passthrough", function () {
          togglePassthrough();
        });
      }
    } catch (e) {
      console.error("event listen failed:", e);
    }

    // Enable passthrough by default on startup
    invoke("set_passthrough", { enabled: true })
      .then(function () {
        passthroughEnabled = true;
        updatePassthroughUI();
      })
      .catch(function () {});
  }

  // ─── Context Menu ────────────────────────────────────────────────
  canvas.addEventListener("contextmenu", function (e) {
    e.preventDefault();
    // Grab focus so keyboard shortcuts work
    window.focus();
    updatePassthroughUI();
    menu.style.display = "block";
    var menuH = menu.offsetHeight;
    if (!menuH) menuH = 80;
    var menuW = 170;
    var x = e.clientX;
    var y = e.clientY;
    if (x + menuW > window.innerWidth) x = window.innerWidth - menuW - 4;
    if (x < 4) x = 4;
    if (y + menuH > window.innerHeight && y - menuH > 0) {
      y = y - menuH;
    } else if (y + menuH > window.innerHeight) {
      y = window.innerHeight - menuH - 4;
    }
    if (y < 4) y = 4;
    menu.style.left = x + "px";
    menu.style.top = y + "px";
  });

  menu.addEventListener("click", function (e) {
    var item = e.target.closest(".item");
    if (!item) return;
    menu.style.display = "none";

    var state = item.dataset.state;

    if (state === "passthrough") {
      togglePassthrough();
    } else if (state === "exit") {
      if (hasTauri && window.__haloTriggerExit) {
        window.__haloTriggerExit(function () {
          invoke("plugin:window|close");
        });
      } else if (hasTauri) {
        invoke("plugin:window|close");
      }
      return;
    }
  });

  document.addEventListener("click", function (e) {
    if (!menu.contains(e.target)) { menu.style.display = "none"; }
    if (!summary.contains(e.target)) { summary.style.display = "none"; }
  });

  // ─── Left Click: Session Summary ─────────────────────────────────
  canvas.addEventListener("click", function (e) {
    if (e.button !== 0) return;
    if (Math.abs(e.clientX - dragStartX) > 3 || Math.abs(e.clientY - dragStartY) > 3) return;
    updateSummary();
    var x = e.clientX,
      y = e.clientY;
    var sumW = 220,
      sumH = 100;
    if (x + sumW > window.innerWidth) x = window.innerWidth - sumW - 8;
    if (y + sumH > window.innerHeight) y = y - sumH - 8;
    if (x < 4) x = 4;
    if (y < 4) y = 4;
    summary.style.left = x + "px";
    summary.style.top = y + "px";
    summary.style.display = "block";
  });

  // ─── Drag ────────────────────────────────────────────────────────
  var dragStartX = 0,
    dragStartY = 0;
  var isDragging = false;

  canvas.addEventListener("mousedown", function (e) {
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    isDragging = false;
  });

  canvas.addEventListener("mousemove", function (e) {
    if (e.buttons !== 1) return;
    if (Math.abs(e.clientX - dragStartX) < 3 && Math.abs(e.clientY - dragStartY) < 3) return;
    if (!isDragging) {
      isDragging = true;
      canvas.classList.add("dragging");
      if (hasTauri) {
        invoke("plugin:window|start_dragging").catch(function () {});
      }
    }
  });

  canvas.addEventListener("mouseup", function () {
    canvas.classList.remove("dragging");
  });

  window.addEventListener("mouseup", function () {
    canvas.classList.remove("dragging");
  });

  // ─── Keyboard (local, only when window has focus) ──────────────
  window.addEventListener("keydown", function (e) {
    if (e.key === "Escape") {
      menu.style.display = "none";
      summary.style.display = "none";
    }
  });
})();
