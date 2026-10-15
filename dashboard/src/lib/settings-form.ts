import { invoke } from "@tauri-apps/api/core";
import type { DashboardConfig } from "./config";
import { prepareConfigForSave } from "./prepare-config";

export const SAVED_NOTICE = "Saved. The daemon reads this config on its next start.";

export async function saveDashboardConfig(
  config: DashboardConfig,
): Promise<{ ok: true; config: DashboardConfig; notice: string } | { ok: false; error: string }> {
  const prepared = prepareConfigForSave(config);
  if (!prepared.ok) return prepared;
  try {
    await invoke("save_config", { config: prepared.config });
  } catch (caught) {
    const error = caught instanceof Error ? caught.message : String(caught);
    return { ok: false, error };
  }
  return { ok: true, config: prepared.config, notice: SAVED_NOTICE };
}
