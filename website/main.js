const repo = 'UmarErth/ForgeIDE';
const releaseUrl = `https://github.com/${repo}/releases/latest`;
const targets = {
  windows: { id: 'download-windows', patterns: ['windows-setup.exe', 'windows-portable.exe', 'windows.msi'] },
  macos: { id: 'download-macos', patterns: ['macos.dmg', 'macos.app.zip'] },
  linux: { id: 'download-linux', patterns: ['linux-x86_64.appimage'] }
};

async function connectDownloads() {
  try {
    const response = await fetch(`https://api.github.com/repos/${repo}/releases/latest`, { headers: { Accept: 'application/vnd.github+json' } });
    if (!response.ok) return;
    const release = await response.json();
    for (const platform of Object.values(targets)) {
      const asset = platform.patterns.map(pattern => release.assets.find(item => item.name.toLowerCase().endsWith(pattern))).find(Boolean);
      if (asset) document.getElementById(platform.id).href = asset.browser_download_url;
    }
    const version = document.getElementById('version-label');
    if (version) version.textContent = `${release.tag_name.toUpperCase()} · OPEN SOURCE · GEMINI`;
    const note = document.getElementById('release-note');
    if (note) note.textContent = `Latest release ${release.tag_name} · published ${new Date(release.published_at).toLocaleDateString()}`;
  } catch {
    const note = document.getElementById('release-note');
    if (note) note.textContent = 'Browse all builds on GitHub Releases.';
  }
}

connectDownloads();
