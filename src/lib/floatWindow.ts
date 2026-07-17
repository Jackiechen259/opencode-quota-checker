import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { load as loadStore } from "@tauri-apps/plugin-store";

export const FLOAT_LABEL = "float";

const STORE_PATH = "settings.json";
const KEY_FLOAT_COMPACT = "float_compact";

/** 完整版 / 精简版窗口尺寸 */
export const FLOAT_SIZES = {
  full: { width: 344, height: 404 },
  compact: { width: 344, height: 128 },
  docked: { width: 344, height: 52 },
};

export interface FloatWindowCreateOptions {
  width?: number;
  height?: number;
  x?: number;
  y?: number;
}

/** 读取持久化的精简模式偏好(默认完整版) */
export async function getCompactPref(): Promise<boolean> {
  try {
    const store = await loadStore(STORE_PATH);
    return (await store.get<boolean>(KEY_FLOAT_COMPACT)) ?? false;
  } catch {
    return false;
  }
}

/** 持久化精简模式偏好 */
export async function setCompactPref(value: boolean): Promise<void> {
  try {
    const store = await loadStore(STORE_PATH);
    await store.set(KEY_FLOAT_COMPACT, value);
    await store.save();
  } catch {
    /* ignore */
  }
}

/**
 * 创建悬浮小窗(若已存在则直接返回该实例)。
 * 无边框、始终置顶、不在任务栏显示,可由用户拖动。
 * 创建时按上次偏好选择完整版或精简版尺寸,避免切换闪烁。
 */
export async function createFloatWindow(
  opts: FloatWindowCreateOptions = {}
): Promise<WebviewWindow> {
  const existing = await getFloatWindow();
  if (existing) return existing;

  const compact = await getCompactPref();
  const size = compact ? FLOAT_SIZES.compact : FLOAT_SIZES.full;

  return new WebviewWindow(FLOAT_LABEL, {
    url: "index.html?window=float",
    title: "方舟配额",
    width: opts.width ?? size.width,
    height: opts.height ?? size.height,
    minWidth: 200,
    minHeight: 40,
    x: opts.x,
    y: opts.y,
    resizable: true,
    decorations: false,
    transparent: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    visible: true,
  });
}

/** 获取已存在的悬浮窗实例,不存在返回 null */
export async function getFloatWindow(): Promise<WebviewWindow | null> {
  return await WebviewWindow.getByLabel(FLOAT_LABEL);
}
