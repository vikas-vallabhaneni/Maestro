(function () {
  "use strict";

  var rescanBtn = document.getElementById("rescan-btn");
  var indicator = document.getElementById("scan-indicator");
  var trackList = document.getElementById("track-list");

  function formatDuration(ms) {
    if (ms == null) return "";
    var totalSec = Math.floor(ms / 1000);
    var min = Math.floor(totalSec / 60);
    var sec = totalSec % 60;
    return min + ":" + (sec < 10 ? "0" : "") + sec;
  }

  function renderTracks(tracks) {
    trackList.innerHTML = "";
    for (var i = 0; i < tracks.length; i++) {
      var t = tracks[i];
      var tr = document.createElement("tr");
      tr.innerHTML =
        '<td class="num">' + (t.track_no != null ? t.track_no : "") + "</td>" +
        "<td>" + escapeHtml(t.title || "") + "</td>" +
        "<td>" + escapeHtml(t.artist || "") + "</td>" +
        "<td>" + escapeHtml(t.album || "") + "</td>" +
        '<td class="num">' + formatDuration(t.duration_ms) + "</td>";
      trackList.appendChild(tr);
    }
  }

  function escapeHtml(str) {
    var div = document.createElement("div");
    div.appendChild(document.createTextNode(str));
    return div.innerHTML;
  }

  function fetchTracks() {
    fetch("/api/tracks")
      .then(function (r) { return r.json(); })
      .then(renderTracks)
      .catch(function (err) { console.error("Failed to fetch tracks:", err); });
  }

  function pollScan(scanId) {
    rescanBtn.disabled = true;

    var poll = setInterval(function () {
      fetch("/api/scans/" + scanId)
        .then(function (r) { return r.json(); })
        .then(function (scan) {
          indicator.textContent = "Scanning… " + scan.files_seen + " files";
          if (scan.finished_at) {
            clearInterval(poll);
            indicator.textContent = "";
            rescanBtn.disabled = false;
            fetchTracks();
          }
        })
        .catch(function () {
          clearInterval(poll);
          indicator.textContent = "";
          rescanBtn.disabled = false;
        });
    }, 500);
  }

  rescanBtn.addEventListener("click", function () {
    rescanBtn.disabled = true;
    fetch("/api/scans", { method: "POST" })
      .then(function (r) { return r.json(); })
      .then(function (data) { pollScan(data.id); })
      .catch(function (err) {
        console.error("Failed to start scan:", err);
        rescanBtn.disabled = false;
      });
  });

  fetchTracks();
})();
