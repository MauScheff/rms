import { execFileSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { rulesBridgeFailed, rulesBridgeSucceeded } from "./representation.mjs";

const CURRENT_DIR = dirname(fileURLToPath(import.meta.url));
const DEFAULT_BRIDGE_MANIFEST = resolve(CURRENT_DIR, "../rules-bridge/Cargo.toml");

export function createRulesEffectExecutor(options = {}) {
  const run = options.run ?? execFileSync;
  const manifestPath = options.manifestPath ?? DEFAULT_BRIDGE_MANIFEST;
  return Object.freeze({
    execute(effect) {
      if (effect.tag !== "InvokeRulesBridge") {
        return rulesBridgeFailed("unsupported-effect");
      }
      const args = [
        "run",
        "--quiet",
        "--manifest-path",
        manifestPath,
        "--",
        ...effect.acceptedMoveIndexes.map(String),
        String(effect.move.cell.index),
      ];
      try {
        return rulesBridgeSucceeded(
          Object.freeze(JSON.parse(run("cargo", args, { encoding: "utf8" }))),
        );
      } catch (error) {
        return rulesBridgeFailed(error instanceof Error ? error.message : String(error));
      }
    },
  });
}

export function createRulesPort(overrides = {}) {
  if (overrides.execute) return Object.freeze({ execute: overrides.execute });
  if (overrides.applyMove) {
    return Object.freeze({
      execute(effect) {
        try {
          return rulesBridgeSucceeded(overrides.applyMove(effect.move));
        } catch (error) {
          return rulesBridgeFailed(error instanceof Error ? error.message : String(error));
        }
      },
    });
  }
  return createRulesEffectExecutor(overrides);
}
