#!/usr/bin/env node
// `pnpm dev`: runs `tauri dev` presenting the version the pending changesets will release
// (QUESTKEEP_APP_VERSION, read by debug builds only), so plugins that need the next version work
// without editing any version file. Extra arguments go to `tauri dev`.
import { execFileSync, spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const env = { ...process.env };
if (!env.QUESTKEEP_APP_VERSION) {
  try {
    env.QUESTKEEP_APP_VERSION = execFileSync(
      process.execPath,
      [path.join(root, "scripts", "release", "changeset.mjs"), "dev-version"],
      { cwd: root, encoding: "utf8" },
    ).trim();
  } catch (err) {
    console.warn(`could not work out the next version, using the released one: ${err.message}`);
  }
}
if (env.QUESTKEEP_APP_VERSION) console.log(`Dev build presents version ${env.QUESTKEEP_APP_VERSION}`);

const child = spawn(["pnpm tauri dev", ...process.argv.slice(2)].join(" "), {
  cwd: root,
  env,
  stdio: "inherit",
  shell: true,
});
child.on("exit", (code) => process.exit(code ?? 1));
