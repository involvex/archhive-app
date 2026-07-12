import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const ROOT = path.resolve(import.meta.dir, "..");
const SEMVER = /^\d+\.\d+\.\d+(-[\w.-]+)?$/;

async function main(): Promise<void> {
  const version = process.argv[2];
  if (!version || !SEMVER.test(version)) {
    console.error("Usage: bun scripts/bump-version.ts <semver>");
    console.error("Example: bun scripts/bump-version.ts 0.2.0");
    process.exit(1);
  }

  const packagePath = path.join(ROOT, "package.json");
  const tauriPath = path.join(ROOT, "src-tauri", "tauri.conf.json");
  const cargoPath = path.join(ROOT, "src-tauri", "Cargo.toml");

  const pkg = JSON.parse(await readFile(packagePath, "utf8")) as { version: string };
  pkg.version = version;
  await writeFile(packagePath, `${JSON.stringify(pkg, null, 2)}\n`, "utf8");

  const tauri = JSON.parse(await readFile(tauriPath, "utf8")) as { version: string };
  tauri.version = version;
  await writeFile(tauriPath, `${JSON.stringify(tauri, null, 2)}\n`, "utf8");

  let cargo = await readFile(cargoPath, "utf8");
  cargo = cargo.replace(/^version = ".*"$/m, `version = "${version}"`);
  await writeFile(cargoPath, cargo, "utf8");

  console.log(`Bumped version to ${version} in package.json, tauri.conf.json, Cargo.toml`);
}

await main();
