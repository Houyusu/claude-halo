// Claude Halo — App Logic (minimal: state sync only)
(function () {
  "use strict";

  var currentState = "idle";
  var hasTauri = typeof window.__TAURI_INTERNALS__ !== "undefined";
  var invoke = hasTauri ? window.__TAURI_INTERNALS__.invoke : null;

  function setState(state) {
    if (state === currentState) return;
    currentState = state;
    window.__haloState = state;
  }

  if (hasTauri) {
    // Polling fallback (1500ms — lightweight, event listener is primary)
    setInterval(function () {
      invoke("get_state")
        .then(function (state) { setState(state); })
        .catch(function () {});
    }, 1500);

    // Tauri event listener (primary, near-instant)
    try {
      var tauriEvent = (window.__TAURI__ && window.__TAURI__.event)
        || (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.event);
      if (tauriEvent) {
        tauriEvent.listen("state-changed", function (event) {
          setState(event.payload);
        });
      }
    } catch (e) {}
  }
})();
