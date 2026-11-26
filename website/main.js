const repo = 'UmarErth/ForgeIDE';
const releasesUrl = `https://github.com/${repo}/releases`;
const latestUrl = `${releasesUrl}/latest`;

const platforms = {
  windows: {
    label: 'Windows',
    button: 'download-windows',
    patterns: ['forgeide-windows-setup.exe', 'forgeide-windows.msi', 'forgeide-windows-portable.exe']
  },
  macos: {
    label: 'macOS',
    button: 'download-macos',
    patterns: ['forgeide-macos.dmg', 'forgeide-macos.app.zip']
  },
  linux: {
    label: 'Linux',
    button: 'download-linux',
    patterns: ['forgeide-linux-x86_64.appimage']
  }
};

function detectPlatform() {
  const platform = (navigator.userAgentData?.platform || navigator.platform || navigator.userAgent || '').toLowerCase();
  if (platform.includes('win')) return 'windows';
  if (platform.includes('mac') || platform.includes('darwin')) return 'macos';
  if (platform.includes('linux') || platform.includes('x11')) return 'linux';
  return null;
}

function findAsset(release, patterns) {
  for (const pattern of patterns) {
    const asset = release.assets.find(item => item.name.toLowerCase() === pattern);
    if (asset) return asset;
  }
  return null;
}

async function connectDownloads() {
  const primary = document.getElementById('download-primary');
  const detected = detectPlatform();
  if (primary && detected) {
    primary.href = latestUrl;
    primary.setAttribute('aria-label', `Download Forge IDE for ${platforms[detected].label}`);
    primary.innerHTML = `Download for ${platforms[detected].label} <span>↓</span>`;
  }

  try {
    const response = await fetch(`https://api.github.com/repos/${repo}/releases/latest`, {
      headers: { Accept: 'application/vnd.github+json' }
    });
    if (!response.ok) throw new Error('No published release yet.');
    const release = await response.json();

    for (const [key, platform] of Object.entries(platforms)) {
      const asset = findAsset(release, platform.patterns);
      const button = document.getElementById(platform.button);
      if (button && asset) button.href = asset.browser_download_url;
    }

    const recommendedAsset = detected ? findAsset(release, platforms[detected].patterns) : null;
    if (primary) {
      if (recommendedAsset) {
        primary.href = recommendedAsset.browser_download_url;
        primary.setAttribute('download', '');
      } else {
        primary.href = latestUrl;
        primary.removeAttribute('download');
      }
    }

    const version = document.getElementById('version-label');
    if (version) version.textContent = `${release.tag_name.toUpperCase()} · OPEN SOURCE · GEMINI`;
    const note = document.getElementById('release-note');
    if (note) {
      note.textContent = release.assets.length
        ? `Latest release ${release.tag_name} · choose your platform above`
        : `Release ${release.tag_name} has no downloadable files yet.`;
    }
  } catch {
    const note = document.getElementById('release-note');
    if (note) note.textContent = 'No downloads published yet. Check back soon or view GitHub Releases.';
    if (primary) primary.href = releasesUrl;
    for (const platform of Object.values(platforms)) {
      const button = document.getElementById(platform.button);
      if (button) button.href = releasesUrl;
    }
  }
}

connectDownloads();
