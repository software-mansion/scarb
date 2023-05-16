import { Octokit } from "@octokit/core";
import { components } from "@octokit/openapi-types";
import semver from "semver";

if (typeof window !== "undefined") {
  throw new Error("@/data/github module is server-side only");
}

const octokit = new Octokit();

export type GHRelease = components["schemas"]["release"];
export type GHAsset = components["schemas"]["release-asset"];

export interface Release {
  version: string;
  assets: Asset[];
}

export type AssetKind = "Archive" | "Source" | "Installer" | "Checksums";

export interface Asset {
  name: string;
  kind: AssetKind;
  os: string;
  arch: string;
  downloadUrl: string;
}

export class Releases {
  private constructor(private list: GHRelease[]) {}

  public static async list(): Promise<Releases> {
    const list = await listReleasesFromRecent();
    return new Releases(list);
  }

  public stable(): Release | null {
    return (
      viewRelease(this.list.find((r) => !r.draft && !r.prerelease)) ?? null
    );
  }

  public preview(): Release | null {
    const preview = viewRelease(this.list.find((r) => r.prerelease));
    if (!preview) {
      return null;
    }

    const stable = this.stable();
    if (!stable || semver.lt(stable.version, preview.version)) {
      return preview;
    } else {
      return null;
    }
  }
}

function viewRelease(release: GHRelease | undefined): Release | undefined {
  if (!release) {
    return release;
  }

  const version = semver.clean(release.tag_name);
  if (!version) {
    throw new Error(`release tag is not valid semver: '${release.tag_name}'`);
  }

  return {
    version,
    assets: collectAssets(release),
  };
}

function collectAssets(release: GHRelease): Asset[] {
  let assets = [...release.assets];

  if (release.tarball_url) {
    assets.push(
      makeSourceGHAsset(
        "scarb",
        release.tag_name,
        release.tarball_url,
        "tar.gz"
      )
    );
  }

  if (release.zipball_url) {
    assets.push(
      makeSourceGHAsset("scarb", release.tag_name, release.zipball_url, "zip")
    );
  }

  const viewedAssets = assets.map(viewAsset);
  viewedAssets.sort((a, b) => assetSortKey(a).localeCompare(assetSortKey(b)));
  return viewedAssets;
}

function viewAsset(asset: GHAsset): Asset {
  const platform =
    (/((?:x86|aarch|arm)[^.]+)/.exec(asset.name) ?? [null, null])[1] ?? "";

  const fileext =
    (/\.(sh|exe|pkg|sha\d*)$/.exec(asset.name) ?? [null, null])[1] ?? "";

  let kind: AssetKind = "Archive";
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

function makeSourceGHAsset(
  repo: string,
  tag_name: string,
  url: string,
  filetype: string
): GHAsset {
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

function assetSortKey(asset: Asset): string {
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

async function listReleasesFromRecent(): Promise<GHRelease[]> {
  const response = await octokit.request("GET /repos/{owner}/{repo}/releases", {
    owner: "software-mansion",
    repo: "scarb",
    headers: {
      "X-GitHub-Api-Version": "2022-11-28",
    },
  });
  return response.data;
}
