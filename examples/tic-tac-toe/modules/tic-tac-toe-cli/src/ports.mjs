import { execFileSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const CURRENT_DIR = dirname(fileURLToPath(import.meta.url));
const DEFAULT_BRIDGE_MANIFEST = resolve(CURRENT_DIR, "../rules-bridge/Cargo.toml");

export function createRulesPort(overrides = {}) {
  if (!overrides.applyMove) {
    return createRustRulesPort(overrides);
  }
  return Object.freeze({
    applyMove: overrides.applyMove,
  });
}

export function createRustRulesPort(options = {}) {
  const run = options.run ?? execFileSync;
  const manifestPath = options.manifestPath ?? DEFAULT_BRIDGE_MANIFEST;
  const acceptedMoveIndexes = [];

  return Object.freeze({
    applyMove(move) {
      const args = [
        "run",
        "--quiet",
        "--manifest-path",
        manifestPath,
        "--",
        ...acceptedMoveIndexes.map(String),
        String(move.cell.index),
      ];
      const output = run("cargo", args, { encoding: "utf8" });
      const result = Object.freeze(JSON.parse(output));
      if (result.outcome?.tag === "Accepted") {
        acceptedMoveIndexes.push(move.cell.index);
      }
      return result;
    },
    acceptedMoveIndexes() {
      return Object.freeze([...acceptedMoveIndexes]);
    },
  });
}
