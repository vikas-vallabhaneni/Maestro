import { html, render, useState, useEffect, useRef } from './vendor/preact.module.js';

function formatDuration(ms) {
  if (ms == null) return '';
  const s = Math.floor(ms / 1000);
  return Math.floor(s / 60) + ':' + String(s % 60).padStart(2, '0');
}

function groupByArtist(tracks) {
  const map = new Map();
  for (const t of tracks) {
    const artist = t.artist || 'Unknown Artist';
    const album = t.album || 'Unknown Album';
    if (!map.has(artist)) map.set(artist, new Map());
    const albums = map.get(artist);
    if (!albums.has(album)) albums.set(album, []);
    albums.get(album).push(t);
  }
  return map;
}

function sortTracks(tracks) {
  return [...tracks].sort((a, b) => {
    if (a.track_no != null && b.track_no != null) return a.track_no - b.track_no;
    if (a.track_no != null) return -1;
    if (b.track_no != null) return 1;
    return (a.title || '').localeCompare(b.title || '');
  });
}

function TrackRow({ track, isPlaying, onPlay }) {
  return html`
    <tr class=${isPlaying ? 'playing' : ''} onClick=${() => onPlay(track)}>
      <td class="num">${track.track_no != null ? track.track_no : ''}</td>
      <td>${track.title || ''}</td>
      <td class="num">${formatDuration(track.duration_ms)}</td>
    </tr>`;
}

function TrackList({ tracks, playingTrackId, onPlay }) {
  const sorted = sortTracks(tracks);
  return html`
    <table>
      <thead><tr>
        <th class="num">#</th>
        <th>Title</th>
        <th class="num">Duration</th>
      </tr></thead>
      <tbody>
        ${sorted.map(t => html`<${TrackRow} key=${t.id} track=${t}
          isPlaying=${t.id === playingTrackId} onPlay=${onPlay} />`)}
      </tbody>
    </table>`;
}

function NowPlaying({ track, audioRef }) {
  return html`
    <footer class="now-playing">
      <div class="now-playing-info">
        ${track && html`${track.title || ''}${track.artist
          ? html` · <span class="accent">${track.artist}</span>` : ''}`}
      </div>
      <audio ref=${audioRef} controls />
    </footer>`;
}

function App() {
  const [tracks, setTracks] = useState([]);
  const [state, setState] = useState({ selectedAlbumId: null, playingTrackId: null });
  const [expandedArtist, setExpandedArtist] = useState(null);
  const [scanning, setScanning] = useState(null);
  const audioRef = useRef(null);

  const fetchTracks = () =>
    fetch('/api/tracks').then(r => r.json()).then(setTracks)
      .catch(e => console.error('Failed to fetch tracks:', e));

  useEffect(() => { fetchTracks(); }, []);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio || !state.playingTrackId) return;
    audio.src = '/api/tracks/' + state.playingTrackId + '/stream';
    audio.play().catch(() => {});
  }, [state.playingTrackId]);

  const grouped = groupByArtist(tracks);
  const artists = [...grouped.keys()].sort((a, b) => a.localeCompare(b));

  let selectedTracks = [];
  if (state.selectedAlbumId) {
    const sep = state.selectedAlbumId.indexOf('\0');
    const artist = state.selectedAlbumId.slice(0, sep);
    const album = state.selectedAlbumId.slice(sep + 1);
    selectedTracks = grouped.get(artist)?.get(album) || [];
  }

  const playingTrack = tracks.find(t => t.id === state.playingTrackId) || null;

  const play = t => setState(s => ({ ...s, playingTrackId: t.id }));
  const selectAlbum = (artist, album) =>
    setState(s => ({ ...s, selectedAlbumId: artist + '\0' + album }));
  const toggleArtist = artist =>
    setExpandedArtist(a => a === artist ? null : artist);

  const rescan = () => {
    fetch('/api/scans', { method: 'POST' }).then(r => r.json()).then(data => {
      setScanning(0);
      const poll = setInterval(() => {
        fetch('/api/scans/' + data.id).then(r => r.json()).then(scan => {
          setScanning(scan.files_seen);
          if (scan.finished_at) {
            clearInterval(poll);
            setScanning(null);
            fetchTracks();
          }
        }).catch(() => { clearInterval(poll); setScanning(null); });
      }, 500);
    }).catch(e => console.error('Failed to start scan:', e));
  };

  return html`
    <header>
      <h1>Maestro</h1>
      <div class="scan-controls">
        ${scanning != null
          && html`<span class="scan-indicator">Scanning… ${scanning} files</span>`}
        <button onClick=${rescan} disabled=${scanning != null}>Rescan</button>
      </div>
    </header>
    <main>
      <nav class="sidebar">
        ${artists.map(artist => html`
          <div key=${artist}>
            <div class="artist-name" onClick=${() => toggleArtist(artist)}>
              <span class="arrow">${expandedArtist === artist ? '▾' : '▸'}</span>
              ${' ' + artist}
            </div>
            ${expandedArtist === artist && html`
              <div class="album-list">
                ${[...grouped.get(artist).keys()].map(album => html`
                  <div key=${album}
                    class=${'album-name' + (state.selectedAlbumId === artist + '\0' + album
                      ? ' selected' : '')}
                    onClick=${() => selectAlbum(artist, album)}>
                    ${album}
                  </div>`)}
              </div>`}
          </div>`)}
      </nav>
      <section class="content">
        ${tracks.length === 0
          ? html`<div class="empty">Library is empty</div>`
          : !state.selectedAlbumId
            ? html`<div class="empty">Select an album</div>`
            : html`<${TrackList} tracks=${selectedTracks}
                playingTrackId=${state.playingTrackId} onPlay=${play} />`}
      </section>
    </main>
    <${NowPlaying} track=${playingTrack} audioRef=${audioRef} />`;
}

render(html`<${App} />`, document.getElementById('app'));
