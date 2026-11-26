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
    button: 'download-macos-arm64',
    patterns: ['forgeide-macos-arm64.dmg', 'forgeide-macos-arm64.app.zip']
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

async function detectMacArchitecture() {
  const hints = navigator.userAgentData;
  if (hints?.getHighEntropyValues) {
    try {
      const info = await hints.getHighEntropyValues(['architecture']);
      const architecture = (info.architecture || '').toLowerCase();
      if (architecture.includes('arm')) return 'arm64';
      if (architecture === 'x86' || architecture.includes('x86_64')) return 'x86_64';
    } catch {}
  }
  const agent = (navigator.userAgent || '').toLowerCase();
  if (agent.includes('arm64') || agent.includes('aarch64')) return 'arm64';
  if (agent.includes('x86_64')) return 'x86_64';
  return null;
}

const macAssets = {
  arm64: ['forgeide-macos-arm64.dmg', 'forgeide-macos-arm64.app.zip'],
  x86_64: ['forgeide-macos-x86_64.dmg', 'forgeide-macos-x86_64.app.zip']
};

async function connectDownloads() {
  const primary = document.getElementById('download-primary');
  const detected = detectPlatform();\n  const macArchitecture = detected === 'macos' ? await detectMacArchitecture() : null;
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
    for (const [arch, patterns] of Object.entries(macAssets)) {
      const asset = findAsset(release, patterns);
      const button = document.getElementById(arch === 'arm64' ? 'download-macos-arm64' : 'download-macos-x86_64');
      if (button && asset) button.href = asset.browser_download_url;
    }

    const recommendedPatterns = detected === 'macos' ? (macArchitecture ? macAssets[macArchitecture] : []) : detected ? platforms[detected].patterns : [];\n    const recommendedAsset = recommendedPatterns.length ? findAsset(release, recommendedPatterns) : null;
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
    document.getElementById('download-macos-arm64')?.setAttribute('href', releasesUrl);
    document.getElementById('download-macos-x86_64')?.setAttribute('href', releasesUrl);
  }
}

connectDownloads();
