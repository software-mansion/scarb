import { Octokit } from "@octokit/core";
import semver from "semver";
import { cached } from "./.vitepress/data/cache";

const octokit = new Octokit({
  // We only authenticate to avoid rate limits on GitHub API requests.
  // All resources we download are public and providing the token is not required.
  auth: process.env.GITHUB_TOKEN || undefined,
});

export default {
  async load() {
    const all = await cached("scarb-github-releases", listReleasesFromRecent);
    const stable = findStableRelease(all);
    const preview = findPreviewRelease(all);

    const stableFull = viewFullRelease(
      await cached("scarb-github-release-stable-full", () =>
        getRelease(stable.id),
      ),
    );

    return {
      stable: { ...stable, ...stableFull },
      preview,
      latestVersion: stable?.version,
      sampleVersion: preview?.version ?? stable?.version,
    };
  },
};

function findStableRelease(list) {
  return viewRelease(list.find((r) => !r.draft && !r.prerelease));
}

function findPreviewRelease(list) {
  const preview = viewRelease(list.find((r) => r.prerelease));
  if (!preview) {
    return null;
  }

  const stable = findStableRelease(list);
  if (!stable || semver.lt(stable.version, preview.version)) {
    return preview;
  } else {
    return null;
  }
}

function viewRelease(release) {
  if (!release) {
    return release;
  }

  const version = semver.clean(release.tag_name);
  if (!version) {
    throw new Error(`release tag is not valid semver: '${release.tag_name}'`);
  }

  return {
    id: release.id,
    version,
    assets: collectAssets(release),
  };
}

function collectAssets(release) {
  let assets = [...release.assets];

  if (release.tarball_url) {
    assets.push(
      makeSourceGHAsset(
        "scarb",
        release.tag_name,
        release.tarball_url,
        "tar.gz",
      ),
    );
  }

  if (release.zipball_url) {
    assets.push(
      makeSourceGHAsset("scarb", release.tag_name, release.zipball_url, "zip"),
    );
  }

  const viewedAssets = assets.map(viewAsset);
  viewedAssets.sort((a, b) => assetSortKey(a).localeCompare(assetSortKey(b)));
  return viewedAssets;
}

function viewAsset(asset) {
  const platform =
    (/((?:x86|aarch|arm)[^.]+)/.exec(asset.name) ?? [null, null])[1] ?? "";

  const fileext =
    (/\.(sh|exe|pkg|sha\d*)$/.exec(asset.name) ?? [null, null])[1] ?? "";

  let kind = "Archive";
  if (fileext === "sh" || fileext === "exe" || fileext === "pkg") {
    kind = "Installer";
  } else if (fileext.startsWith("sha")) {
    kind = "Checksums";
  } else if (!platform) {
    kind = "Source";
  }

  let os;
  if (platform.includes("apple-darwin")) {
    os = "macOS";
  } else if (platform.includes("windows")) {
    os = "Windows";
  } else if (platform.includes("linux")) {
    os = "Linux";
  } else {
    os = "";
  }

  const arch = platform.split("-")[0];

  return {
    name: asset.name,
    kind,
    os,
    arch,
    downloadUrl: asset.browser_download_url,
  };
}

function makeSourceGHAsset(repo, tag_name, url, filetype) {
  return {
    id: 0,
    name: `${repo}-${tag_name}.${filetype}`,
    url,
    browser_download_url: url,
    content_type: "",
    created_at: "",
    download_count: 0,
    label: null,
    node_id: "",
    size: 0,
    state: "uploaded",
    updated_at: "",
    // @ts-ignore
    uploader: {},
  };
}

function assetSortKey(asset) {
  let kind;
  switch (asset.kind) {
    case "Installer":
      kind = 0;
      break;
    case "Archive":
      kind = 1;
      break;
    case "Source":
      kind = 2;
      break;
    case "Checksums":
      kind = 3;
      break;
  }

  return `${kind}-${asset.os}-${asset.name}`;
}

function viewFullRelease(release) {
  if (!release) {
    return release;
  }

  const cairoVersion = extractCairoVersionFromReleaseNotes(release.body);

  return {
    cairoVersion,
    starknetPackageVersionReq: `>=${cairoVersion}`,
  };
}

function extractCairoVersionFromReleaseNotes(body) {
  const match = body.match(
    /\[`v.+`]\(https:\/\/github.com\/starkware-libs\/cairo\/releases\/tag\/v([0-9a-zA-Z.-]+)\)/,
  );

  if (!match) {
    throw new Error(
      `Failed to extract Cairo version from release notes:\n\n${body}`,
    );
  }

  return match[1];
}

async function listReleasesFromRecent() {
  const response = await octokit.request("GET /repos/{owner}/{repo}/releases", {
    owner: "software-mansion",
    repo: "scarb",
    headers: {
      "X-GitHub-Api-Version": "2022-11-28",
    },
  });
  return response.data;
}

async function getRelease(releaseId) {
  const response = await octokit.request(
    "GET /repos/{owner}/{repo}/releases/{release_id}",
    {
      owner: "software-mansion",
      repo: "scarb",
      release_id: releaseId,
      headers: {
        "X-GitHub-Api-Version": "2022-11-28",
      },
    },
  );
  return response.data;
}
