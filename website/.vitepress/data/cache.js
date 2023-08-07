import path from "path";
import fs from "fs/promises";

const CACHE_DIR = (async () => {
  const p = path.join(".vitepress", "cache", "scarb");
  await fs.mkdir(p, { recursive: true });
  return p;
})();

export async function cached(key, fn) {
  const cacheFile = path.join(await CACHE_DIR, `${key}.json`);

  try {
    const json = await fs.readFile(cacheFile, { encoding: "utf-8" });
    const data = JSON.parse(json);
    console.info(`using cached data of: ${key}, source file: ${cacheFile}`);
    return data;
  } catch (_e) {
    console.info(`cache miss: ${key}`);
  }

  const data = await fn();
  const json = JSON.stringify(data, null, 2);
  await fs.writeFile(cacheFile, json);

  return data;
}
